use ::syn::{
    Token, Type, TypePath,
    parse::{self, Parse, ParseStream},
};

use ::proc_macro2::Ident;

/// Helper struct for parsing the `load!` macro input
pub struct LoadInput {
    pub lua_expr: Ident,
    pub type_paths: Vec<TypePath>,
}

/// Custom parser for `lua, MyStruct, MyEnum, ...`
impl Parse for LoadInput {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let lua_expr: Ident = input.parse()?;
        let mut type_paths: Vec<TypePath> = Vec::new();

        // Continue parsing idents as long as there's a comma
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }; // Allow trailing comma
            // Get the type path

            if let Type::Path(type_path) = input.parse()? {
                type_paths.push(type_path);
            } else {
                panic!("Expected a type path");
            };
        }

        return Ok(Self {
            lua_expr: lua_expr,
            type_paths: type_paths,
        });
    }
}
