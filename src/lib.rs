mod compile;

extern crate proc_macro;

use ::proc_macro::TokenStream;

use ::proc_macro2;
use ::proc_macro2::{Ident, };

use ::quote::quote;

use ::syn::{
	parse_macro_input,
	Fields,
	ImplItem, ItemEnum,
	Pat,
};

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
/// #[mlua_magic_macros::structure]
/// #[derive(Clone)]
/// struct Player {
///     name: String,
///     hp: i32,
/// }
///
/// // Later, compile userdata:
/// mlua_magic_macros::compile!(Player, fields, methods);
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

	/*let fields = match &ast.fields {
		Data::Struct(DataStruct {
			fields: Fields::Named(FieldsNamed { named, .. }),
			..
		}) => named,
		_ => panic!("#[mlua_magic::structure] only works on structs with named fields."), // TODO: Implement this.
	};*/
	// ^^^^
	// TODO: Add type validation?
	let mut user_data_fields = Vec::new();

	for field in &ast.fields {
		let field_name: &Ident = field.ident.as_ref().expect("Field must have a name");
		let field_name_str: String = field_name.to_string();
		// let field_ty: &syn::Type = &field.ty;

		user_data_fields.push(quote! {
			fields.add_field_method_get(#field_name_str, |_, this| {
				return Ok(this.#field_name.clone());
			});
		});

		/*user_data_fields.push(quote! {
			fields.add_field_method_set(#field_name_str, |_, this, val: #field_ty| {
				this.#field_name = val;
				return Ok(());
			});
		});*/
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

	let original_tokens: proc_macro2::TokenStream = quote! { #ast };
	let helper_tokens: proc_macro2::TokenStream = quote! { #helper_fn };

	let mut output: proc_macro2::TokenStream = original_tokens;
	output.extend(helper_tokens);

	return output.into();
}

/// Implements a helper function `_to_mlua_variants` for an enum.
///
/// This function registers all *unit variants* (e.g., `MyEnum::VariantA`)
/// as static properties on the Lua UserData. This allows accessing
/// them in Lua as `MyEnum.VariantA`.
///
/// Variants with data (e.g., `MyEnum::VariantB(i32)`) are *not*
/// automatically registered. You should expose these by creating
/// a static constructor function in an `#[mlua_magic::implementation]`
/// block.
///
/// # Example:
/// ```ignore
/// #[mlua_magic::enumeration]
/// #[derive(Clone, Copy)] // Often needed for UserData enums
/// enum MyEnum {
///	 VariantA,
///	 VariantB(i32),
/// }
///
/// #[mlua_magic::implementation]
/// impl MyEnum {
///	 // This will expose `MyEnum.new_variant_b(123)` in Lua
///	 pub fn new_variant_b(val: i32) -> Self {
///		 Self::VariantB(val)
///	 }
/// }
/// ```
///
/// This is intended to be used with `impl mlua::UserData`.
#[proc_macro_attribute]
pub fn enumeration(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let ast: ItemEnum = parse_macro_input!(item as ItemEnum);
	let name: &Ident = &ast.ident;
	let name_str: String = name.to_string();

	// Build registrations for unit variants (register as static constructors)
	let mut variant_registrations: Vec<proc_macro2::TokenStream> = Vec::new();
	for variant in &ast.variants {
		if let Fields::Unit = &variant.fields {
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
	}

	// Create helper fn _to_mlua_variants, plus FromLua and IntoLua impls for lossless userdata round-trip.
	// FromLua requires Clone so we can return owned values from borrowed userdata.
	let helper_fn: proc_macro2::TokenStream = quote! {
		impl #name {
			#[doc(hidden)]
			pub fn _to_mlua_variants<M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
				#(#variant_registrations)*
			}
		}

		// Convert Lua -> Rust enum (from_lua). We accept userdata and borrow it, then clone.
		impl mlua::FromLua for #name where #name: Clone {
			fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
				match value {
					mlua::Value::UserData(ud) => {
						// Attempt to borrow the inner enum; clone and return owned value.
						let borrowed = ud.borrow::<#name>()?;
						Ok(borrowed.clone())
					},
					other => Err(mlua::Error::FromLuaConversionError {
						from: other.type_name(),
						to: #name_str.to_string(),
						message: Some(format!("expected userdata for {}", stringify!(#name))),
					})
				}
			}
		}
	};

	let original_tokens: proc_macro2::TokenStream = quote! { #ast };
	let helper_tokens: proc_macro2::TokenStream = quote! { #helper_fn };

	let mut output: proc_macro2::TokenStream = original_tokens;
	output.extend(helper_tokens);

	return output.into();
}


/// Generates a helper function `_to_mlua_methods` for an `impl` block.
///
/// This function registers all methods in the `impl` block with mlua,
/// correctly distinguishing between static, `&self`, and `&mut self` methods.
#[proc_macro_attribute]
pub fn implementation(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let ast: syn::ItemImpl = parse_macro_input!(item as syn::ItemImpl);
	let name: &Box<syn::Type> = &ast.self_ty;

	let mut method_registrations: Vec<proc_macro2::TokenStream> = Vec::new();

	for item in &ast.items {
		if let ImplItem::Fn(fn_item) = item {
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

			// Check for `&self`, `&mut self`, or static
			if let Some(receiver) = &fn_item.sig.receiver() {
				if receiver.mutability.is_some() {
					// Here, `this`` is is `&mut self`
					method_registrations.push(quote! {
						methods.add_method_mut(#fn_name_str, |_, this, (#(#arg_names,)*): (#(#arg_tys,)*)| {
							Ok(this.#fn_name(#(#arg_names,)*))
						});
					}.into());
				} else {
					// Here, `this`` is `&self`
					method_registrations.push(quote! {
						 methods.add_method(#fn_name_str, |_, this, (#(#arg_names,)*): (#(#arg_tys,)*)| {
							Ok(this.#fn_name(#(#arg_names,)*))
						});
					}.into());
				};
			} else {
				// This is a static function (like `new`)
				method_registrations.push(quote! {
					methods.add_function(#fn_name_str, |_, (#(#arg_names,)*): (#(#arg_tys,)*)| {
						Ok(#name::#fn_name(#(#arg_names,)*))
					});
				}.into());
			};
		};
	};

	// Create the helper function `_to_mlua_methods`
	let helper_fn: proc_macro2::TokenStream = quote! {
		impl #name {
			#[doc(hidden)]
			pub fn _to_mlua_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
				#(#method_registrations)*
			}
		}
	};

	let original_tokens: proc_macro2::TokenStream = quote! { #ast };
	let helper_tokens: proc_macro2::TokenStream = quote! { #helper_fn };

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
///	 // ... methods ...
/// }
///
/// // Generates `impl mlua::UserData for Player`
/// mlua_magic::compile!(Player, fields, methods);
/// ```
///
/// # Example (for an enum):
/// ```ignore
/// #[mlua_magic::enumeration]
/// enum Status { Idle, Busy }
///
/// #[mlua_magic::implementation]
/// impl Status {
///	 // ... methods ...
/// }
///
/// // Generates `impl mlua::UserData for Status` and `impl mlua::IntoLua for Status`
/// mlua_magic::compile!(Status, variants, methods);
/// ```
#[proc_macro]
pub fn compile(item: TokenStream) -> TokenStream {
	let compile::CompileInput { type_name, helpers } = parse_macro_input!(item as compile::CompileInput);

	let mut has_fields: bool = false;
	let mut has_methods: bool = false;
	let mut has_variants: bool = false;

	// Check which helpers the user specified
	for helper in helpers {
		let h: String = helper.to_string();
		if h == "fields" { 
			has_fields = true; 
		} else if h == "methods" { 
			has_methods = true; 
		} else if h == "variants" { 
			has_variants = true; 
		} else {
			// Return a compile error if the helper name is unknown
			return syn::Error::new(helper.span(), "Unknown helper: expected 'fields', 'methods', or 'variants'")
				.to_compile_error()
				.into();
		};
	};

	// Conditionally generate the call to the helper function
	let fields_call = if has_fields {
		quote! {
			Self::_to_mlua_fields(fields);
		}
	} else {
		quote! { /* Do nothing */ }
	};

	let methods_call = if has_methods {
		quote! {
			Self::_to_mlua_methods(methods);
		}
	} else {
		quote! { /* Do nothing */ }
	};

	let variants_call = if has_variants {
		quote! {
			Self::_to_mlua_variants(methods);
		}
	} else {
		quote! { /* Do nothing */ }
	};

	// Assemble the final `impl mlua::UserData` block
	let output: proc_macro2::TokenStream = quote! {
		impl mlua::UserData for #type_name {
			fn add_fields<'lua, F: mlua::UserDataFields<Self>>(fields: &mut F) -> () {
				#fields_call
			}

			fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) -> () {
				#methods_call
				#variants_call
			}
		}
		/*impl mlua::IntoLua for #type_name {
			fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
				let user_data: mlua::AnyUserData = lua.create_any_userdata(self)?;
				let value: mlua::Value = user_data.to_value();

				return Ok(value);
			}
		}*/
	};

	println!("{}", output);

	return output.into();
}