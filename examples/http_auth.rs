extern crate byteorder;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate extprim;
extern crate futures;
extern crate hyper;
extern crate log;
extern crate mtproto;
extern crate rand;
extern crate select;
extern crate serde;
extern crate serde_mtproto;
extern crate tokio_core;


use std::str;

use byteorder::{ByteOrder, BigEndian};
use futures::{Future, Stream};
use mtproto::tl::dynamic::TLObject;
use mtproto::rpc::{AppInfo, Session};
use mtproto::rpc::encryption::asymm;
use mtproto::rpc::message::{Message, MessageType};
use mtproto::schema;
use rand::Rng;
use select::document::Document;
use select::predicate::Name;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_core::reactor::{Core, Handle};


mod error {
    error_chain! {
        links {
            MtProto(::mtproto::Error, ::mtproto::ErrorKind);
            SerdeMtProto(::serde_mtproto::Error, ::serde_mtproto::ErrorKind);
        }

        foreign_links {
            Hyper(::hyper::Error);
            Io(::std::io::Error);
            SetLogger(::log::SetLoggerError);
            Utf8(::std::str::Utf8Error);
        }

        errors {
            NonceMismatch(expected: ::extprim::i128::i128, found: ::extprim::i128::i128) {
                description("nonce mismatch")
                display("nonce mismatch (expected {}, found {})", expected, found)
            }

            HtmlErrorText(error_text: String) {
                description("RPC returned an HTML error")
                display("RPC returned an HTML error with text: {}", error_text)
            }

            BadMessage(found_len: usize) {
                description("Message is not HTML error and is < 24 bytes long")
                display("Message is not HTML error and is {} < 24 bytes long", found_len)
            }

            UnknownHtmlErrorStructure(html: String) {
                description("Unknown HTML error structure")
                display("Unknown HTML error structure:\n{}", html)
            }
        }
    }
}

use error::{ErrorKind, ResultExt};

macro_rules! bailf {
    ($e:expr) => {
        return Box::new(futures::future::err($e.into()))
    }
}

macro_rules! tryf {
    ($e:expr) => {
        match { $e } {
            Ok(v) => v,
            Err(e) => bailf!(e),
        }
    }
}


fn auth(handle: Handle) -> Box<Future<Item = (), Error = error::Error>> {
    let app_info = tryf!(AppInfo::read_from_toml_file("AppInfo.toml")
        .chain_err(|| "this example needs a AppInfo.toml file with `api_id` and `api_hash` fields in it"));

    let http_client = hyper::Client::new(&handle);

    let mut rng = rand::thread_rng();
    let mut session = Session::new(rng.gen(), app_info);

    let nonce = rng.gen();
    let req_pq = schema::rpc::req_pq {
        nonce: nonce,
    };

    let http_request = tryf!(create_http_request(&mut session, req_pq, MessageType::PlainText));
    let auth_future = future_request(&http_client, http_request).and_then(move |response_bytes|
        -> Box<Future<Item = (Vec<u8>, Session), Error = error::Error>>
    {
        let response: Message<schema::ResPQ> = tryf!(parse_response(&mut session, &response_bytes));

        let res_pq = response.unwrap_plain_text_body();

        if nonce != res_pq.nonce {
            bailf!(ErrorKind::NonceMismatch(nonce, res_pq.nonce));
        }

        let pq_u64 = BigEndian::read_u64(&res_pq.pq);
        println!("Decomposing pq = {}...", pq_u64);
        let (p_u32, q_u32) = tryf!(asymm::decompose_pq(pq_u64));
        println!("Decomposed p = {}, q = {}", p_u32, q_u32);
        let u32_to_vec = |num| {
            let mut v = vec![0; 4];
            BigEndian::write_u32(v.as_mut_slice(), num);
            v
        };
        let p = u32_to_vec(p_u32);
        let q = u32_to_vec(q_u32);

        let p_q_inner_data = schema::P_Q_inner_data::p_q_inner_data(schema::p_q_inner_data {
            pq: res_pq.pq,
            p: p.clone().into(),
            q: q.clone().into(),
            nonce: res_pq.nonce,
            server_nonce: res_pq.server_nonce,
            new_nonce: rng.gen(),
        });

        println!("Data to send: {:#?}", &p_q_inner_data);
        let p_q_inner_data_serialized = tryf!(serde_mtproto::to_bytes(&p_q_inner_data));
        println!("Data bytes to send: {:?}", &p_q_inner_data_serialized);
        let known_sha1_fingerprints = tryf!(asymm::KNOWN_RAW_KEYS.iter()
            .map(|raw_key| {
                let sha1_fingerprint = raw_key.read()?.sha1_fingerprint()?;
                Ok(sha1_fingerprint.iter().map(|b| format!("{:02x}", b)).collect::<String>())
            })
            .collect::<error::Result<Vec<_>>>());
        println!("Known public key SHA1 fingerprints: {:?}", known_sha1_fingerprints);
        let known_fingerprints = tryf!(asymm::KNOWN_RAW_KEYS.iter()
            .map(|raw_key| Ok(raw_key.read()?.fingerprint()?))
            .collect::<error::Result<Vec<_>>>());
        println!("Known public key fingerprints: {:?}", known_fingerprints);
        let server_pk_fingerprints = res_pq.server_public_key_fingerprints.inner().as_slice();
        println!("Server public key fingerprints: {:?}", &server_pk_fingerprints);
        let (rsa_public_key, fingerprint) =
            tryf!(asymm::find_first_key_fail_safe(server_pk_fingerprints));
        println!("RSA public key used: {:#?}", &rsa_public_key);
        let encrypted_data = tryf!(rsa_public_key.encrypt(&p_q_inner_data_serialized));
        println!("Encrypted data: {:?}", encrypted_data.as_ref());
        let encrypted_data2 = tryf!(rsa_public_key.encrypt2(&p_q_inner_data_serialized));
        println!("Encrypted data 2: {:?}", &encrypted_data2);

        let req_dh_params = schema::rpc::req_DH_params {
            nonce: res_pq.nonce,
            server_nonce: res_pq.server_nonce,
            p: p.into(),
            q: q.into(),
            public_key_fingerprint: fingerprint,
            encrypted_data: encrypted_data.to_vec().into(),
            //encrypted_data: encrypted_data2.into(),
        };

        let http_request = tryf!(create_http_request(&mut session, req_dh_params, MessageType::PlainText));

        Box::new(future_request(&http_client, http_request).map(|bytes| (bytes, session)))
    }).and_then(|(response_bytes, mut session)| {
        let _: Message<schema::Server_DH_Params> = tryf!(parse_response(&mut session, &response_bytes));

        Box::new(futures::future::ok(()))
    });

    Box::new(auth_future)
}

fn parse_response<T>(session: &mut Session, response_bytes: &[u8]) -> error::Result<Message<T>>
    where T: ::std::fmt::Debug + DeserializeOwned
{
    println!("Response bytes: {:?}", &response_bytes);

    if let Ok(response_str) = str::from_utf8(response_bytes) {
        let response_str = response_str.trim();
        let str_len = response_str.len();

        if str_len >= 7 && &response_str[0..6] == "<html>" && &response_str[str_len-7..] == "</html>" {
            let response_str = str::from_utf8(response_bytes)?;
            let doc = Document::from(response_str);
            println!("HTML error response:\n{}", response_str);

            let error_text = match doc.find(Name("h1")).next() {
                Some(elem) => elem.text(),
                None => bail!(ErrorKind::UnknownHtmlErrorStructure(response_str.to_owned())),
            };

            bail!(ErrorKind::HtmlErrorText(error_text));
        }
    }

    let len = response_bytes.len();

    if len < 24 {
        bail!(ErrorKind::BadMessage(len));
    }

    // We're being simplistic here, i.e. we actually should pass None if the message is plain-text
    let encrypted_data_len = Some((len - 24) as u32);
    let response = session.process_message(&response_bytes, encrypted_data_len)?;
    println!("Message received: {:#?}", &response);

    Ok(response)
}

fn create_http_request<T>(session: &mut Session,
                          data: T,
                          message_type: MessageType)
                         -> error::Result<hyper::Request>
    //where T: ::std::fmt::Debug + Serialize + Identifiable + MtProtoSized
    where T: ::std::fmt::Debug + Serialize + TLObject
{
    let message = session.create_message(data, message_type)?;
    println!("Message to send: {:#?}", &message);
    let serialized_message = serde_mtproto::to_bytes(&message)?;
    println!("Request bytes: {:?}", &serialized_message);

    // Here we do mean to unwrap since it should fail if something goes wrong anyway
    //assert_eq!(message.size_hint().unwrap(), serialized_message.len());

    let mut request = hyper::Request::new(
        hyper::Method::Post,
        "http://149.154.167.51:443/api".parse().unwrap(),
    );

    request
        .headers_mut()
        .set(hyper::header::Connection::keep_alive());
    request
        .headers_mut()
        .set(hyper::header::ContentLength(serialized_message.len() as u64));

    //println!("{:?}", &serialized_message);
    request.set_body(serialized_message);

    Ok(request)
}

fn future_request(http_client: &hyper::Client<hyper::client::HttpConnector>,
                  http_request: hyper::Request)
                 -> Box<Future<Item = Vec<u8>, Error = error::Error>> {
    let future = http_client
        .request(http_request)
        .and_then(|res| res.body().concat2())
        .map(|data| data.to_vec())
        .map_err(|err| err.into());

    Box::new(future)
}


fn run() -> error::Result<()> {
    env_logger::init()?;
    let mut core = Core::new()?;

    let auth_future = auth(core.handle());
    core.run(auth_future)?;

    Ok(())
}

quick_main!(run);
