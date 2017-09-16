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
        NoSalts {}
        NoField {}
        // other
        FactorizationFailure {}
        NoAuthKey {}
        ErrorCode(code: i32) {}
        BadMessage(len: usize) {}
    }
}
