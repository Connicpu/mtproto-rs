error_chain! {
    errors {
        WrongTyParamsCount(ty_params: Vec<::quote::Tokens>, needed_count: usize) {
            description("wrong number of type parameters")
            display("wrong number of type parameters: {:?} (need {}, found {})",
                &ty_params, needed_count, ty_params.len())
        }
    }
}
