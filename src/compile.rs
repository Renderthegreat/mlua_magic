use ::syn::{
	parse::{self, Parse, ParseStream, },
	Token,
};

use ::proc_macro2::{Ident, };

/// Helper struct for parsing the `compile!` macro input
pub struct CompileInput {
    pub type_name: Ident,
    pub helpers: Vec<Ident>,
}

/// Custom parser for `TypeName, helper1, helper2, ...`
impl Parse for CompileInput {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let type_name: Ident = input.parse()?;
        let mut helpers: Vec<Ident> = Vec::new();

        // Continue parsing idents as long as there's a comma
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() { break; } // Allow trailing comma
            helpers.push(input.parse()?);
        };

        Ok(CompileInput { type_name, helpers })
    }
}