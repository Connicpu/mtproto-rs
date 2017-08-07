extern crate byteorder;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate futures;
extern crate futures_cpupool;
extern crate mtproto;
extern crate openssl;
extern crate rand;

use byteorder::{BigEndian, ByteOrder};
use futures::{Async, Future, Poll};
use futures_cpupool::CpuPool;
use mtproto::rpc::encryption::{AesParams, AuthKey, asymm};
use mtproto::schema as tl;
use mtproto::tl::serialize_message;
use rand::Rand;
use std::io;

pub mod error {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Canceled(::futures::Canceled);
            Openssl(::openssl::error::ErrorStack);
        }
        links {
            Mtproto(::mtproto::error::Error, ::mtproto::error::ErrorKind);
        }
        errors {
            NoUsableServerKey {}
        }
    }

    pub type BoxFuture<T> = Box<::futures::Future<Item = T, Error = Error>>;
}

use error::{BoxFuture, Error, ErrorKind, Result};

pub trait TelegramClient: Clone {
    fn send_and_await_reply_plain<M>(&mut self, message: M) -> BoxFuture<M::Reply>
        where M: mtproto::rpc::RpcFunction + 'static;
}

fn deserialize_message<M>(reader: &mut io::Read) -> mtproto::error::Result<M>
    where M: mtproto::tl::ReadType,
{
    use mtproto::tl::parsing::{ReadContext, Reader};
    Ok(ReadContext::new(reader).read_tl::<M>()?)
}

#[derive(Debug, Clone)]
struct AwaitingDHParams {
    nonce: tl::Int128,
    server_nonce: tl::Int128,
    aes: AesParams,
    new_nonce: (tl::Int128, tl::Int128),
}

#[derive(Debug, Clone)]
struct AwaitingDHFinal {
    new_nonce: (tl::Int128, tl::Int128),
    new_salt: i64,
    auth_key: AuthKey,
}

enum KexState {
    AwaitingPQ(tl::Int128, BoxFuture<tl::ResPQ>),
    AwaitingDHParams(BoxFuture<(AwaitingDHParams, tl::Server_DH_Params)>),
    AwaitingDHFinal(AwaitingDHFinal, BoxFuture<tl::Set_client_DH_params_answer>),
}

#[derive(Debug, Clone)]
pub struct NewKey {
    pub key: AuthKey,
    pub salt: tl::FutureSalt,
}

struct InnerKexNegotiator<Rng, Client>
    where Rng: rand::Rng,
          Client: TelegramClient,
{
    csrng: Rng,
    client: Client,
    expires_in: Option<i32>,
}

pub struct KexNegotiator<Rng, Client>
    where Rng: rand::Rng,
          Client: TelegramClient,
{
    state: KexState,
    inner: InnerKexNegotiator<Rng, Client>,
}

impl<Rng, Client> KexNegotiator<Rng, Client>
    where Rng: rand::Rng,
          Client: TelegramClient + 'static,
{
    pub fn new(csrng: Rng, client: Client, expires_in: Option<i32>) -> Self {
        let mut inner = InnerKexNegotiator { csrng, client, expires_in };
        KexNegotiator {
            state: inner.send_pq_request(),
            inner: inner,
        }
    }
}

impl<Rng, Client> InnerKexNegotiator<Rng, Client>
    where Rng: rand::Rng,
          Client: TelegramClient + 'static,
{
    fn send_pq_request(&mut self) -> KexState {
        let nonce = tl::Int128::rand(&mut self.csrng);
        KexState::AwaitingPQ(
            nonce, self.client.send_and_await_reply_plain(tl::rpc::req_pq { nonce: nonce }))
    }

    fn send_dh_request(&mut self, prev: tl::Int128, res: tl::ResPQ) -> KexState {
        assert!(prev == res.nonce);
        let pq = BigEndian::read_u64(&res.pq);
        let new_nonce = <(tl::Int128, tl::Int128)>::rand(&mut self.csrng);
        KexState::AwaitingDHParams(Box::new({
            let expires_in = self.expires_in;
            let mut client = self.client.clone();
            CpuPool::new(1)
                .spawn_fn(move || asymm::decompose_pq(pq))
                .map_err(Into::into)
                .and_then(move |(p, q)| prepare_dh_request(new_nonce, p, q, expires_in, res))
                .and_then(move |(next_state, msg)| {
                    client.send_and_await_reply_plain(msg)
                        .map(move |resp| (next_state, resp))
                })
        }))
    }

    fn send_dh_params(&mut self, prev: AwaitingDHParams, res: tl::Server_DH_Params) -> Result<KexState> {
        let encrypted = match res {
            tl::Server_DH_Params::server_DH_params_ok(p) => p.encrypted_answer,
            _ => unimplemented!(),
        };
        let decrypted = prev.aes.ige_decrypt(&encrypted)?;
        let (sha_part, data_part) = decrypted.split_at(20);
        let mut curs = io::Cursor::new(&data_part);
        let res = deserialize_message::<tl::Server_DH_inner_data>(&mut curs)?;
        let inner_hash = sha1(&data_part[..curs.position() as usize])?;
        assert_eq!(&sha_part[..], &inner_hash[..]);
        let (auth_key, g_b) = asymm::calculate_auth_key(res.g as u32, &res.dh_prime, &res.g_a)?;
        let inner = serialize_message(tl::Client_DH_Inner_Data {
            nonce: prev.nonce,
            server_nonce: prev.server_nonce,
            retry_id: 0,
            g_b: g_b,
        })?;
        Ok(KexState::AwaitingDHFinal(AwaitingDHFinal {
            new_nonce: prev.new_nonce,
            new_salt: (prev.new_nonce.0).0 ^ prev.server_nonce.0,
            auth_key: auth_key,
        }, self.client.send_and_await_reply_plain(tl::rpc::set_client_DH_params {
            nonce: prev.nonce,
            server_nonce: prev.server_nonce,
            encrypted_data: prev.aes.ige_encrypt(&inner, true)?,
        })))
    }

    fn parse_dh_final(&mut self, prev: &AwaitingDHFinal, res: tl::Set_client_DH_params_answer) -> Result<NewKey> {
        let expected_new_nonce_hash1 = prev.auth_key.new_nonce_hash(1, prev.new_nonce)?;
        use mtproto::schema::Set_client_DH_params_answer::*;
        match res {
            dh_gen_ok(ref n) if n.new_nonce_hash1 == expected_new_nonce_hash1 => (),
            _ => panic!("mismatch"),
        }
        Ok(NewKey {
            key: prev.auth_key.clone(),
            salt: tl::FutureSalt::from_negotiated_salt(prev.new_salt),
        })
    }
}

fn prepare_dh_request(new_nonce: tl::Int256, p: u32, q: u32, expires_in_opt: Option<i32>, res: tl::ResPQ)
                      -> Result<(AwaitingDHParams, tl::rpc::req_DH_params)> {
    let (pubkey, fingerprint) = match asymm::find_first_key(&res.server_public_key_fingerprints)? {
        Some(t) => t,
        None => return Err(ErrorKind::NoUsableServerKey.into()),
    };
    let (p, q) = {
        let mut p_vec = vec![0u8; 4];
        BigEndian::write_u32(&mut p_vec, p);
        let mut q_vec = vec![0u8; 4];
        BigEndian::write_u32(&mut q_vec, q);
        (p_vec, q_vec)
    };
    let inner = if let Some(expires_in) = expires_in_opt {
        tl::P_Q_inner_data::p_q_inner_data_temp(tl::p_q_inner_data_temp {
            pq: res.pq.clone(),
            p: p.clone(),
            q: q.clone(),
            nonce: res.nonce,
            server_nonce: res.server_nonce,
            new_nonce: new_nonce,
            expires_in: expires_in,
        })
    } else {
        tl::P_Q_inner_data::p_q_inner_data(tl::p_q_inner_data {
            pq: res.pq.clone(),
            p: p.clone(),
            q: q.clone(),
            nonce: res.nonce,
            server_nonce: res.server_nonce,
            new_nonce: new_nonce,
        })
    };
    Ok((AwaitingDHParams {
        nonce: res.nonce,
        server_nonce: res.server_nonce,
        aes: AesParams::from_pq_inner_data(&inner)?,
        new_nonce: inner.new_nonce(),
    }, tl::rpc::req_DH_params {
        nonce: res.nonce,
        server_nonce: res.server_nonce,
        p: p,
        q: q,
        public_key_fingerprint: fingerprint,
        encrypted_data: pubkey.encrypt(&serialize_message(inner)?)?,
    }))
}

impl<Rng, Client> Future for KexNegotiator<Rng, Client>
    where Rng: rand::Rng,
          Client: TelegramClient + 'static,
{
    type Item = NewKey;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::KexState::*;
        loop {
            self.state = match self.state {
                AwaitingPQ(nonce, ref mut f) =>
                    self.inner.send_dh_request(nonce, try_ready!(f.poll())),
                AwaitingDHParams(ref mut f) => {
                    let (prev, res) = try_ready!(f.poll());
                    self.inner.send_dh_params(prev, res)?
                },
                AwaitingDHFinal(ref dh, ref mut f) => {
                    let key = self.inner.parse_dh_final(dh, try_ready!(f.poll()))?;
                    return Ok(Async::Ready(key));
                },
            }
        }
    }
}

fn sha1(input: &[u8]) -> Result<openssl::hash::DigestBytes> {
    let mut hasher = openssl::hash::Hasher::new(openssl::hash::MessageDigest::sha1())?;
    hasher.update(input)?;
    Ok(hasher.finish2()?)
}
