use std::borrow::BorrowMut;

use convert_case::{Case, Casing};
use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Field, Ident, ItemStruct, Type, Visibility};

#[derive(Debug, FromMeta)]
struct MacroArgs {
    #[darling(default)]
    create: bool,
    #[darling(default)]
    read: bool,
    #[darling(default)]
    readall: bool,
    #[darling(default)]
    update: bool,
    #[darling(default)]
    destroy: bool,
}

static HELPER_ATTR: &str = "petelib";

///
/// Ideally, we'd be able to generate something like this:
///
/// ```
/// #[derive(Debug, Deserialize, Serialize, UsualModel)]
/// #[petelib(create, read, readall, update, destroy)]
/// struct User {
///     #[petelib(readonly)]
///     id: Uuid,
///     email: Option<String>,
///     #[petelib(readonly)]
///     created_at: DateTime<Utc>
/// }
/// ```
///
pub fn petelib(args: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };

    let return_me_later = item.clone();
    let mut input = syn::parse_macro_input!(item as ItemStruct);
    let item = return_me_later;

    let args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let mut ro_fields = vec![];
    let mut secure_fields = vec![];
    let mut all_fields = vec![];
    let mut id_field = None;
    let mut index_fields = vec![];
    let mut queryable_fields = vec![];
    for field in input.fields.iter_mut() {
        if let Some(attr) = field.attrs.iter_mut().find(|attr| match &attr.meta {
            syn::Meta::List(list) => {
                list.path.get_ident().map(ToString::to_string) == Some(HELPER_ATTR.to_string())
            }
            _ => false,
        }) {
            match attr.meta.borrow_mut() {
                syn::Meta::List(ref mut fields) => {
                    let idents = fields
                        .tokens
                        .clone()
                        .into_iter()
                        .filter_map(|v| match v {
                            proc_macro2::TokenTree::Ident(i) => Some(i.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<String>>();

                    if idents.contains(&"id".to_string()) {
                        match id_field {
                            Some(val) => panic!(
                                "You can't define multiple IDs. We already saw {val:?} earlier."
                            ),
                            None => id_field = Some(field.clone()),
                        }
                    }

                    // Remove this attribute
                    field.attrs.retain(|v| {
                        v.meta.path().get_ident().map(ToString::to_string)
                            != Some(HELPER_ATTR.to_string())
                    });

                    if idents.contains(&"secure".to_string()) {
                        secure_fields.push(field.clone());
                    }

                    if idents.contains(&"readonly".to_string()) {
                        ro_fields.push(field.clone());
                    }

                    if idents.contains(&"index".to_string()) {
                        index_fields.push(field.clone());
                    }

                    if idents.contains(&"queryable".to_string()) {
                        queryable_fields.push(field.clone());
                    }
                }
                _ => panic!("petelib should only include single methods"),
            };
        }
        all_fields.push(field.clone());
    }

    let ItemStruct {
        attrs,
        vis,
        struct_token: _,
        ident,
        generics,
        fields: _,
        semi_token,
    } = syn::parse_macro_input!(item as ItemStruct);

    let struct_tokens = quote! {
        #(#attrs)*
        #[derive(UsualModel)]
        #vis struct #ident #generics {
            #(#all_fields),*
        }
        #semi_token
    };

    let secure_struct_tokens = if !secure_fields.is_empty() {
        let non_secure_ident = Ident::new(&format!("NonSecure{ident}"), Span::call_site());
        let non_secure_fields = all_fields
            .clone()
            .into_iter()
            .filter(|v| !secure_fields.contains(v))
            .collect::<Vec<Field>>();
        let non_secure_idents = non_secure_fields.iter().map(|v| v.ident.clone());
        quote! {
            #(#attrs)*
            #[derive(UsualModel)]
            #vis struct #non_secure_ident #generics {
                #(#non_secure_fields),*
            }
            #semi_token

            impl From<#ident> for #non_secure_ident {
                fn from(val: #ident) -> Self {
                    #non_secure_ident {
                        #(
                            #non_secure_idents: val.#non_secure_idents
                        ),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let mut methods = vec![];
    if args.create {
        methods.push(_create(
            ident.clone(),
            all_fields
                .iter()
                .filter(|v| !ro_fields.contains(v))
                .collect::<Vec<&Field>>(),
        ));
    }

    if args.read {
        if let Some(id_field) = id_field.as_ref() {
            methods.push(_read(ident.clone(), &id_field));
        }

        for index_field in index_fields {
            methods.push(_read_index(ident.clone(), &index_field));
        }
    }

    if args.readall {
        methods.push(_readall(ident.clone()));
    }

    for queryable_field in queryable_fields {
        methods.push(_read_queryable(ident.clone(), &queryable_field));
    }

    let update_builder = if args.update {
        let _UpdateStreams {
            update_builder,
            update_fn,
        } = _update(
            ident.clone(),
            all_fields
                .iter()
                .filter(|v| !ro_fields.contains(v))
                .collect::<Vec<&Field>>(),
            &vis.clone(),
        );

        methods.push(update_fn);
        update_builder
    } else {
        quote! {}
    };

    if args.destroy {
        if let Some(id_field) = id_field.as_ref() {
            methods.push(_destroy(ident.clone(), &id_field));
        }
    }

    let impl_tokens = quote! {
        impl #generics #ident #generics {
            #(#vis #methods)*
        }
    };

    let gen = quote! {
        #struct_tokens
        #secure_struct_tokens
        #update_builder

        #impl_tokens
    };

    use rust_format::{Config, Formatter, PostProcess, RustFmt};
    let config = Config::new_str().post_proc(PostProcess::ReplaceMarkersAndDocBlocks);
    proc_macro::Span::call_site()
        .note("Thruster code output")
        .note(
            RustFmt::from_config(config)
                .format_tokens(gen.clone())
                .unwrap_or_else(|_| gen.to_string()),
        )
        .emit();

    gen.into()
}

fn _create(name: Ident, fields: Vec<&Field>) -> proc_macro2::TokenStream {
    let field_numbers = 1..(fields.len() + 1);
    let field_names = fields
        .iter()
        .map(|f| f.ident.clone().unwrap().to_string())
        .collect::<Vec<_>>();
    let field_idents = fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone();
            quote! { & #ident }
        })
        .collect::<Vec<_>>();
    let fields_streams = fields.into_iter().map(|f| {
        let name = f.ident.clone().unwrap();
        let ty = f.ty.clone();
        quote! { #name: #ty }
    });

    let query = format!(
        "
        INSERT INTO {}s
            ({})
        VALUES
            ({})
        RETURNING
            {{{}}}",
        name.to_string().to_case(Case::Snake),
        field_names.join(","),
        field_numbers
            .map(|v| format!("${v}"))
            .collect::<Vec<String>>()
            .join(","),
        name
    );

    quote! {
        async fn create(client: &(impl deadpool_postgres::GenericClient + Send), #(#fields_streams),*) -> Result<#name, tokio_postgres::Error> {
            Ok(#name::from_row(
                &client
                    .query_one(
                        usual::query!(#query)
                    .as_str(),
                    &[
                        #( #field_idents,)*
                    ],
                )
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to execute query: {:?}", e)
                    })?
            ))
        }
    }
}

fn _read(name: Ident, id_field: &Field) -> proc_macro2::TokenStream {
    let id_field = id_field.clone();
    let id_field_name = id_field.ident.unwrap();
    let id_field_ty = id_field.ty;

    let query = format!(
        "SELECT {{{}}} FROM {}s WHERE {} = $1",
        name.to_string(),
        name.to_string().to_case(Case::Snake),
        id_field_name.to_string()
    );

    quote! {
        async fn read(client: &(impl deadpool_postgres::GenericClient + Send), #id_field_name: &#id_field_ty) -> Result<#name, tokio_postgres::Error> {
            Ok(#name::from_row(
                &client
                    .query_one(
                        usual::query!(#query)
                    .as_str(),
                    &[
                        &#id_field_name,
                    ],
                )
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to execute query: {:?}", e)
                    })?
            ))
        }
    }
}

fn _read_index(name: Ident, index_field: &Field) -> proc_macro2::TokenStream {
    let id_field = index_field.clone();
    let id_field_name = id_field.ident.unwrap();
    let id_field_ty = id_field.ty;
    let fn_ident = Ident::new(
        &format!("read_by_{}", index_field.ident.as_ref().unwrap()),
        Span::call_site(),
    );

    let query = format!(
        "SELECT {{{}}} FROM {}s WHERE {} = $1",
        name.to_string(),
        name.to_string().to_case(Case::Snake),
        id_field_name.to_string()
    );

    quote! {
        async fn #fn_ident(client: &(impl deadpool_postgres::GenericClient + Send), #id_field_name: &#id_field_ty) -> Result<#name, tokio_postgres::Error> {
            Ok(#name::from_row(
                &client
                    .query_one(
                        usual::query!(#query)
                    .as_str(),
                    &[
                        &#id_field_name,
                    ],
                )
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to execute query: {:?}", e)
                    })?
            ))
        }
    }
}

fn _read_queryable(name: Ident, query_field: &Field) -> proc_macro2::TokenStream {
    let query = format!(
        "SELECT {{{}}} FROM {}s WHERE {} = $1",
        name.to_string(),
        name.to_string().to_case(Case::Snake),
        query_field.ident.clone().unwrap()
    );
    let query_field_ident = query_field.ident.clone().unwrap();
    let query_field_ty = query_field.ty.clone();
    let fn_ident = Ident::new(
        &format!("read_where_{}", query_field.ident.clone().unwrap()),
        Span::call_site(),
    );

    quote! {
        async fn #fn_ident(client: &(impl deadpool_postgres::GenericClient + Send), #query_field_ident: &#query_field_ty) -> Result<Vec<#name>, tokio_postgres::Error> {
            Ok(client
                .query(
                    usual::query!(#query)
                        .as_str(),
                    &[&#query_field_ident],
                )
                .await
                .inspect_err(|e| {
                    tracing::error!("Failed to execute query: {:?}", e)
                })?
                .iter()
                .map(#name::from_row)
                .collect::<Vec<#name>>())
            }
    }
}

fn _readall(name: Ident) -> proc_macro2::TokenStream {
    let query = format!(
        "SELECT {{{}}} FROM {}s",
        name.to_string(),
        name.to_string().to_case(Case::Snake),
    );

    quote! {
        async fn readall(client: &(impl deadpool_postgres::GenericClient + Send)) -> Result<Vec<#name>, tokio_postgres::Error> {
            Ok(client
                .query(
                    usual::query!(#query)
                        .as_str(),
                    &[],
                )
                .await
                .inspect_err(|e| {
                    tracing::error!("Failed to execute query: {:?}", e)
                })?
                .iter()
                .map(#name::from_row)
                .collect::<Vec<#name>>())
        }
    }
}

struct _UpdateStreams {
    update_builder: proc_macro2::TokenStream,
    update_fn: proc_macro2::TokenStream,
}

fn _update(name: Ident, fields: Vec<&Field>, vis: &Visibility) -> _UpdateStreams {
    let query = format!(
        "SELECT {{{}}} FROM {}s",
        name.to_string(),
        name.to_string().to_case(Case::Snake),
    );

    let update_builder_ident = Ident::new(&format!("{name}UpdateBuilder"), Span::call_site());
    let field_streams = fields.iter().map(|v| {
        let ident = v.ident.clone().unwrap();
        let ty = v.ty.clone();
        quote! {
            #ident: Option<#ty>
        }
    });
    let setter_streams = fields.iter().map(|v| {
        let ident = v.ident.clone().unwrap();
        let ty = v.ty.clone();
        let setter_name = Ident::new(&format!("set_{ident}"), Span::call_site());
        quote! {
            #vis fn #setter_name(mut self, val: #ty) -> Self {
                self.#ident = Some(val);
                self
            }
        }
    });
    _UpdateStreams {
        update_builder: quote! {
            #vis struct #update_builder_ident {
                #(
                    #field_streams
                ),*
            }

            impl #update_builder_ident {
                #(
                    #setter_streams
                )*
            }
        },
        update_fn: quote! {
            async fn update(client: &(impl deadpool_postgres::GenericClient + Send)) -> Result<#name, tokio_postgres::Error> {
                Ok(#name::from_row(&client
                    .query_one(
                        usual::query!(#query)
                            .as_str(),
                        &[],
                    )
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to execute query: {:?}", e)
                    })?
                ))
            }
        },
    }
}

fn _destroy(name: Ident, id_field: &Field) -> proc_macro2::TokenStream {
    let id_field = id_field.clone();
    let id_field_name = id_field.ident.unwrap();
    let id_field_ty = id_field.ty;

    let query = format!(
        "DELETE FROM {}s WHERE {} = $1 RETURNING {{{}}}",
        name.to_string().to_case(Case::Snake),
        id_field_name.to_string(),
        name.to_string(),
    );

    quote! {
        async fn destroy(client: &(impl deadpool_postgres::GenericClient + Send), #id_field_name: #id_field_ty) -> Result<#name, tokio_postgres::Error> {
            Ok(#name::from_row(
                &client
                    .query_one(
                        usual::query!(#query)
                    .as_str(),
                    &[
                        &#id_field_name,
                    ],
                )
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to execute query: {:?}", e)
                    })?
            ))
        }
    }
}
