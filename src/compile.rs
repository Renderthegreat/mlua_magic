use ::syn::{
    Result,
    /*parse::{
        self, Parse, ParseStream, Parser
    },*/
    //parse_macro_input,
    //braced::{ },
    //parenthesized::{ },
    //punctuated::{ Punctuated, },
    //Path,
    //Token,
    /*Type,*/
    TypePath,
};

use ::proc_macro::TokenStream;

use ::darling::FromMeta;

#[derive(Debug, FromMeta)]
#[darling(derive_syn_parse)]
pub struct CompileArgs {
    #[darling(default)]
    pub type_path: Option<TypePath>, // ! This isn't optional ;)
    #[darling(default)]
    pub fields: Option<bool>,
    #[darling(default)]
    pub methods: Option<bool>,
    #[darling(default)]
    pub variants: Option<bool>,
}

pub fn parse_compile_args(input: TokenStream) -> Result<CompileArgs> {
    match syn::parse(input) {
        Ok(value) => {
            return Ok(value);
        }
        Err(e) => {
            return Err(e);
        }
    };
}
