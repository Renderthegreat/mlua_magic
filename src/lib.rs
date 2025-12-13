mod compile;
mod load;

extern crate proc_macro;

use ::proc_macro::TokenStream;

use ::proc_macro2;
use ::proc_macro2::Ident;

use ::quote::{format_ident, quote};

use ::syn::{Fields, Pat, TypePath, parse_macro_input};

use crate::compile::parse_compile_args;

/// Implements a helper function `_to_mlua_fields` for a Rust struct,
/// enabling automatic registration of named fields with `mlua::UserData`.
///
/// When applied to a struct, this macro generates an implementation
/// of a private helper function that is later invoked by the
/// `mlua_magic_macros::compile!` macro. This ensures the struct’s fields
/// are visible in Lua as userdata fields.
///
/// # Behavior
/// * Public and private named fields are exported as readable fields in Lua.
/// * Getter methods are automatically generated via `add_field_method_get`.
/// * Fields must implement `Clone` for successful conversion to Lua values.
///
/// # Limitations
/// * Only structs with **named fields** are currently supported.
/// * Setter support is not yet implemented.
///
/// # Usage
/// Apply the macro directly to the struct definition:
///
/// ```ignore
/// #[derive(Clone, Copy, Default)]
/// #[mlua_magic_macros::structure]
/// struct Player {
///     name: String,
///     hp: i32,
/// }
///
/// // Later, compile userdata:
/// mlua_magic_macros::compile!(type_path = Player, fields = true, methods = true);
/// ```
///
/// After registration through `mlua::UserData`,
/// Lua scripts may access the fields:
///
/// ```lua
/// print(player.name)
/// print(player.hp)
/// ```
///
/// This macro is designed to work together with:
/// * `#[mlua_magic_macros::implementation]` — for methods
/// * `#[mlua_magic_macros::enumeration]` — for enum variants
/// * `mlua_magic_macros::compile!` — final hookup to `mlua::UserData`
///
/// This simplifies mlua integration by reducing boilerplate and
/// ensuring a consistent interface between Rust types and Lua scripts.
#[proc_macro_attribute]
pub fn structure(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemStruct = parse_macro_input!(item as syn::ItemStruct);
    let name: &Ident = &ast.ident;

    // TODO: Add type validation?
    let mut user_data_fields: Vec<proc_macro2::TokenStream> = Vec::new();

    for field in &ast.fields {
        let field_name: &Ident = field.ident.as_ref().expect("Field must have a name");
        let field_name_str: String = field_name.to_string();
        let field_ty: &syn::Type = &field.ty;

        user_data_fields.push(quote! {
            fields.add_field_method_get(#field_name_str, |_, this| {
                return Ok(this.#field_name.clone());
            });
        });

        user_data_fields.push(quote! {
            fields.add_field_method_set(#field_name_str, |_, this, val: #field_ty| {
                this.#field_name = val;
                return Ok(());
            });
        });
    }

    // Create the helper function `_to_mlua_fields`
    let helper_fn: proc_macro2::TokenStream = quote! {
        impl #name {
            #[doc(hidden)]
            pub fn _to_mlua_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) -> () {
                #(#user_data_fields)*
            }
        }
    };

    let original_tokens: proc_macro2::TokenStream = quote! {
        #ast
    };
    let helper_tokens: proc_macro2::TokenStream = quote! {
        #helper_fn
    };

    let mut output: proc_macro2::TokenStream = original_tokens;
    output.extend(helper_tokens);

    return output.into();
}

/// Implements a helper function `_to_mlua_variants` for a Rust `enum'.
///
/// This function registers all variants (e.g.,
/// as static properties on the Lua UserData. This allows accessing
/// them in Lua as `MyEnum.VariantA`.
///
/// # Example:
/// ```ignore
/// #[derive(Clone, Copy)] // Required for UserData methods
/// #[mlua_magic::enumeration]
/// enum MyEnum {
///     VariantA,
///     VariantB(i32),
/// }
/// ```
///
/// This is intended to be used with `impl mlua::UserData`.
#[proc_macro_attribute]
pub fn enumeration(__attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemEnum = parse_macro_input!(item as syn::ItemEnum);
    let name: &Ident = &ast.ident;
    // let name_str: String = name.to_string();

    // Build registrations for unit variants (register as static constructors)
    let mut variant_registrations: Vec<proc_macro2::TokenStream> = Vec::new();
    for variant in &ast.variants {
        match &variant.fields {
            Fields::Unit => {
                let variant_name: &Ident = &variant.ident;
                let variant_name_str: String = variant_name.to_string();

                // use add_function to register an associated/static function that returns the enum
                variant_registrations.push(quote! {
                    // e.g. methods.add_function("Idle", |_, (): ()| Ok(PlayerStatus::Idle));
                    methods.add_function(#variant_name_str, |_, (): ()| {
                        Ok(#name::#variant_name)
                    });
                });
            }
            Fields::Unnamed(fields) => {
                let variant_name = &variant.ident;
                let variant_name_str = variant_name.to_string();

                // Extract each field type T1, T2, …
                let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

                let arg_idents: Vec<Ident> = (0..field_types.len())
                    .map(|i: usize| {
                        return format_ident!("arg{}", i);
                    })
                    .collect();

                variant_registrations.push(quote! {
					methods.add_function(#variant_name_str, |_, (#(#arg_idents),*): (#(#field_types),*)| {
						Ok(#name::#variant_name(#(#arg_idents),*))
					});
				});
            }
            Fields::Named(fields) => {
                // Same pattern as unnamed, except wrap into a struct-like variant:
                let variant_name = &variant.ident;
                let variant_name_str = variant_name.to_string();

                let names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let types: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();

                variant_registrations.push(quote! {
                    methods.add_function(#variant_name_str, |_, tbl: mlua::Table| {
                        Ok(#name::#variant_name {
                            #(#names: tbl.get::<_, #types>(stringify!(#names))?),*
                        })
                    });
                });
            }
        };
    }

    // Create helper fn _to_mlua_variants, plus FromLua and IntoLua impls for lossless userdata round-trip.
    // FromLua requires Clone so we can return owned values from borrowed userdata.
    let helper_fn: proc_macro2::TokenStream = quote! {
        impl #name {
            #[doc(hidden)]
            pub fn _to_mlua_variants<M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
                #(#variant_registrations)*;
            }
        }
    };

    let original_tokens: proc_macro2::TokenStream = quote! {
        #ast
    };
    let helper_tokens: proc_macro2::TokenStream = quote! {
        #helper_fn
    };

    let mut output: proc_macro2::TokenStream = original_tokens;
    output.extend(helper_tokens);

    return output.into();
}

/// Implements a helper function `_to_mlua_methods` for a Rust `impl` block,
/// enabling automatic registration of its methods with `mlua::UserData`.
///
/// When applied to an `impl` block, this macro scans for functions and
/// generates an implementation of a private helper function. This function
/// is later invoked by the `mlua_magic_macros::compile!` macro.
///
/// # Behavior
/// * **Static Functions** (e.g., `fn new() -> Self`) are registered as static
///   functions on the userdata, accessible in Lua as `MyType.new()`.
/// * **Immutable Methods** (e.g., `fn my_method(&self)`) are registered as
///   immutable methods, accessible in Lua as `my_instance:my_method()`.
/// * **Mutable Methods** (e.g., `fn my_mut_method(&mut self)`) are registered as
///   mutable methods, accessible in Lua as `my_instance:my_mut_method()`.
///
/// # Usage
/// Apply the macro directly to the `impl` block for the type:
///
/// ```ignore
/// #[mlua_magic_macros::structure]
/// struct Player { hp: i32 }
///
/// #[mlua_magic_macros::implementation]
/// impl Player {
///     pub fn new() -> Self { Self { hp: 100 } }
///     pub fn is_alive(&self) -> bool { self.hp > 0 }
///     pub fn take_damage(&mut self, amount: i32) { self.hp -= amount; }
/// }
///
/// // Later, compile userdata:
/// mlua_magic_macros::compile!(type_path = Player, fields = true, methods = true);
/// ```
///
/// Lua scripts may then call these methods:
///
/// ```lua
/// local p = Player.new()
/// p:take_damage(30)
/// print(p:is_alive())
/// ```
///
/// This macro is designed to work together with:
/// * `#[mlua_magic_macros::structure]` — for fields
/// * `#[mlua_magic_macros::enumeration]` — for enum variants
/// * `mlua_magic_macros::compile!` — final hookup to `mlua::UserData`
#[proc_macro_attribute]
pub fn implementation(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemImpl = parse_macro_input!(item as syn::ItemImpl);
    let name: &syn::Type = &ast.self_ty;

    let mut method_registrations: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in &ast.items {
        if let syn::ImplItem::Fn(fn_item) = item {
            let fn_name: &Ident = &fn_item.sig.ident;
            let fn_name_str: String = fn_name.to_string();

            // Extract argument names and types, skipping the `self` receiver
            let (arg_names, arg_tys): (Vec<_>, Vec<_>) = fn_item
                .sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let syn::FnArg::Typed(pat_type) = arg {
                        if let Pat::Ident(pat_ident) = &*pat_type.pat {
                            Some((&pat_ident.ident, &pat_type.ty))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unzip();

            // Check if the function is async
            let is_async = fn_item.sig.asyncness.is_some();

            // Check for `&self`, `&mut self`, or static
            if let Some(receiver) = &fn_item.sig.receiver() {
                if receiver.mutability.is_some() {
                    // Here, `this` is `&mut self`
                    if is_async {
                        method_registrations.push(quote! {
                            methods.add_async_method_mut(#fn_name_str, |_, mut this, (#(#arg_names,)*): (#(#arg_tys,)*)| async move {
								return Ok(this.#fn_name(#(#arg_names,)*).await);
                            });
                        });
                    } else {
                        method_registrations.push(quote! {
							methods.add_method_mut(#fn_name_str, |_, this, (#(#arg_names,)*): (#(#arg_tys,)*)| {
								return Ok(this.#fn_name(#(#arg_names,)*));
							});
						});
                    }
                } else {
                    // Here, `this` is `&self`
                    if is_async {
                        method_registrations.push(quote! {
                            methods.add_async_method(#fn_name_str, |_, this, (#(#arg_names,)*): (#(#arg_tys,)*)| async move {
                        		return Ok(this.#fn_name(#(#arg_names,)*).await);
                            });
                        });
                    } else {
                        method_registrations.push(quote! {
                            methods.add_method(#fn_name_str, |_, this, (#(#arg_names,)*): (#(#arg_tys,)*)| {
								return Ok(this.#fn_name(#(#arg_names,)*));
							});
						});
                    }
                };
            } else {
                // This is a static function (like `new`)
                if is_async {
                    method_registrations.push(quote! {
                        methods.add_async_function(#fn_name_str, |_, (#(#arg_names,)*): (#(#arg_tys,)*)| async {
                            return Ok(#name::#fn_name(#(#arg_names,)*).await);
                        });
                    });
                } else {
                    method_registrations.push(quote! {
                        methods.add_function(#fn_name_str, |_, (#(#arg_names,)*): (#(#arg_tys,)*)| {
                            return Ok(#name::#fn_name(#(#arg_names,)*));
                        });
                    });
                }
            };
        };
    }

    // Create the helper function `_to_mlua_methods`
    let helper_fn: proc_macro2::TokenStream = quote! {
        impl #name {
            #[doc(hidden)]
            pub fn _to_mlua_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
                #(#method_registrations)*
            }
        }
    };

    let original_tokens: proc_macro2::TokenStream = quote! {
        #ast
    };
    let helper_tokens: proc_macro2::TokenStream = quote! {
        #helper_fn
    };

    let mut output: proc_macro2::TokenStream = original_tokens;
    output.extend(helper_tokens);

    return output.into();
}

// # Bottom of file
// TODO: Move out of lib.rs when possible

/// Generates the final `impl mlua::UserData` block for a type.
///
/// This macro calls the helper functions generated by `#[structure]`,
/// `#[implementation]`, and `#[enumeration]`.
///
/// You must specify which helpers to include.
///
/// # Example (for a struct):
/// ```ignore
/// #[mlua_magic::structure]
/// struct Player { health: i32 }
///
/// #[mlua_magic::implementation]
/// impl Player {
///     // ... methods ...
/// }
///
/// // Generates `impl mlua::UserData for Player`
/// mlua_magic::compile!(type_path = Player, fields = true, methods true);
/// ```
///
/// # Example (for an enum):
/// ```ignore
/// #[mlua_magic::enumeration]
/// enum Status { Idle, Busy }
///
/// #[mlua_magic::implementation]
/// impl Status {
///     // ... methods ...
/// }
///
/// // Generates `impl mlua::UserData for Status` and `impl mlua::IntoLua for Status`
/// mlua_magic::compile!(type_path = Status, variants = true, methods = true);
/// ```
#[proc_macro]
pub fn compile(input: TokenStream) -> TokenStream {
    let compile_args: compile::CompileArgs = parse_compile_args(input).unwrap();
    let type_path: TypePath = compile_args.type_path.clone().expect("Type is required.");

    // Conditionally generate the call to the helper function
    let fields_call: proc_macro2::TokenStream = if compile_args.fields.unwrap_or(false) {
        quote! {
            Self::_to_mlua_fields(fields);
        }
    } else {
        quote! { /* Do nothing */ }
    };

    let methods_call: proc_macro2::TokenStream = if compile_args.methods.unwrap_or(false) {
        quote! {
            Self::_to_mlua_methods(methods);
        }
    } else {
        quote! { /* Do nothing */ }
    };

    let variants_call: proc_macro2::TokenStream = if compile_args.variants.unwrap_or(false) {
        quote! {
            Self::_to_mlua_variants(methods);
        }
    } else {
        quote! { /* Do nothing */ }
    };

    // Assemble the final `impl mlua::UserData` block
    let output: proc_macro2::TokenStream = quote! {
        impl mlua::UserData for #type_path {
            fn add_fields<'lua, F: mlua::UserDataFields<Self>>(fields: &mut F) -> () {
                #fields_call
            }

            fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
                #methods_call
                #variants_call
            }
        }
        impl mlua::FromLua for #type_path {
            fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
                let output: mlua::Result<Self> = match value {
                    mlua::Value::UserData(user_data) => {
                        return match user_data.borrow::<Self>() {
                            Ok(b) => Ok((*b).clone()),
                            Err(_) => Err(mlua::Error::FromLuaConversionError {
                                from: "UserData",
                                to: stringify!(#type_path).to_string(),
                                message: Some("userdata is not this exact Rust type".into()),
                            })
                        };
                    },
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!(#type_path).to_string(),
                        message: Some("expected userdata created by mlua_magic_macros".into()),
                    }),
                };

                return output;
            }
        }
        /*impl #type_path {
            #[doc(hidden)]
            pub fn _to_mlua_skeleton(lua: &mlua::Lua) -> Result<mlua::AnyUserData, mlua::Error> { // Spooky scary skeletons
                let skeleton: mlua::AnyUserData = lua.create_any_userdata(Self::default())?;

                // TODO: Implement this

                return Ok(skeleton);
            }
        }*/
        /*impl mlua::IntoLua for #type_path {
            fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                let user_data: mlua::AnyUserData = lua.create_any_userdata(self)?;
                let value: mlua::Value = user_data.to_value();

                return Ok(value);
            }
        }*/
    };

    return output.into();
}

/// Registers one or more Rust types implementing `mlua::UserData` as global
/// variables in a `mlua::Lua` instance.
///
/// This macro is the final step to make your Rust types accessible from Lua.
/// It creates a "proxy" for each type (which acts as a constructor table)
/// and assigns it to a global variable in Lua with the same name as the Rust type.
///
/// # Usage
/// The macro takes the `lua` instance as the first argument, followed by a
/// comma-separated list of types to register.
///
/// ```ignore
/// // (Assuming Player and PlayerStatus implement mlua::UserData)
/// use mlua::prelude::*;
///
/// fn main() -> LuaResult<()> {
///     let lua = Lua::new();
///
///     // This call...
///     mlua_magic_macros::load!(lua, Player, PlayerStatus);
///
///     // ...is equivalent to this Lua code:
///     // Player = (proxy for Player UserData)
///     // PlayerStatus = (proxy for PlayerStatus UserData)
///
///     lua.load(r#"
///         print(Player)       -- "userdata: Player"
///         print(PlayerStatus) -- "userdata: PlayerStatus"
///
///         local p = Player.new("Hero")
///         p.status = PlayerStatus.Walking()
///     "#).exec()?;
///
///     Ok(())
/// }
/// ```
///
/// # Prerequisites
/// All types passed to `load!` must implement `mlua::UserData`. This is
/// typically handled by using the `mlua_magic_macros::compile!` macro.
#[proc_macro]
pub fn load(input: TokenStream) -> TokenStream {
    let load::LoadInput {
        lua_expr,
        type_paths,
    } = parse_macro_input!(input as load::LoadInput);

    let output: proc_macro2::TokenStream = quote! {{
        let lua: &mlua::Lua = &#lua_expr;
        let globals: mlua::Table = lua.globals();

        #(
            // Register type globally under its Rust name
            globals.set(stringify!(#type_paths), lua.create_proxy::<#type_paths>()?)?;
        )*
    }};

    return output.into();
}
