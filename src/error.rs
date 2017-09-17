error_chain! {
    links {
        SerdeMtProto(::serde_mtproto::Error, ::serde_mtproto::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        FromUtf8(::std::string::FromUtf8Error);
        OpenSsl(::openssl::error::ErrorStack);
    }

    errors {
        InvalidType(expected: Vec<::tl::parsing::ConstructorId>, received: Option<::tl::parsing::ConstructorId>) {}
        AuthKeyTooLong(key_in: Vec<u8>) {}
        WrongFingerprint(fingerprint: i64) {}

        NoServerSalts {
            description("No server salts found in the session")
        }

        NotEnoughFields(label: &'static str, fields_count_so_far: usize) {
            description("Not enough deserialized fields")
            display("Not enough deserialized fields for {}: {} fields deserialized so far",
                label, fields_count_so_far)
        }

        // other
        FactorizationFailure {}
        NoAuthKey {}
        ErrorCode(code: i32) {}
        BadMessage(len: usize) {}
    }
}
