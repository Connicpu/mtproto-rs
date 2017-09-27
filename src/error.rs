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

        Sha1Total255Longer {
            description("The input string is already longer than 255 bytes")
        }

        NoRsaPublicKeyForFingerprints(fingerprints: Vec<i64>) {
            description("No RSA public key found corresponding to any of specified fingerprints")
            display("No RSA public key found corresponding to any of specified fingerprints: {:?}",
                fingerprints)
        }

        NoModulus {
            description("No modulus found from a RSA key")
        }

        NoExponent {
            description("No exponent found from a RSA key")
        }

        // other
        FactorizationFailure {}
        NoAuthKey {}
        ErrorCode(code: i32) {}
        BadMessage(len: usize) {}
    }
}
