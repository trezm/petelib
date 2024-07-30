#[macro_export]
macro_rules! generate_crud {
    (@CREATE $s:ty, $ctx:ty, {$( $key:ident: $value:ty, )* }) => {
        generate_crud!(@CREATE $s, $ctx, { $( $key: $value, )* } -> {} -> {} -> { $( $key: $value, )* });
    };
    (@CREATE $s:ty, $ctx:ty, {$( $req_key:ident: $req_value:ty, )* } -> {$( $opt_key:ident: $opt_value:ty, )* } -> {$( $ctx_key:ident: $ctx_expr:expr, )* } -> {$( $res_key:ident: $res_value:ty, )*}) => {
        paste::paste! {
            #[derive(Debug, serde::Deserialize)]
            pub struct [<Create $s Request>] {
                $(
                    $req_key: $req_value,
                )*
                $(
                    $opt_key: Option<$opt_value>,
                )*
            }

            #[derive(Debug, serde::Serialize)]
            pub struct [<Create $s Response>] {
                $(
                    $res_key: $res_value
                ),*
            }

            #[doc = "Creates a new `" $s "`."]
            #[thruster::json_request]
            pub async fn [< create_ $s:snake >]([< new_ $s:snake _request >]: [< Create $s Request >], mut context: $ctx, _next: MiddlewareNext<$ctx>) -> MiddlewareResult<$ctx> {
                let db: &Arc<prisma::PrismaClient> = context.extra.get();
                let db = db.clone();
                let [< Create $s Request >] {
                    $(
                        $req_key,
                    )*
                    $(
                        $opt_key,
                    )*

                } = [< new_ $s:snake _request >];

                let optional_values = vec![
                    $(
                        if $opt_key.is_some() {
                            Some(prisma::[< $s:snake >]::[< $opt_key >]::set($opt_key.unwrap()))
                        } else {
                            None
                        }
                     ),*
                ]
                    .into_iter()
                    .filter_map(|v| v)
                    .collect();

                let result: prisma::[< $s:snake >]::Data = db.[< $s:snake >]()
                    .create($(
                        $req_key,
                    )* $(
                        ($ctx_expr)(&context),
                    )* optional_values)
                    .exec()
                    .await
                    .expect("Temporary for failures, fix this");

                let response = [< Create $s Response >] {
                    $(
                        $res_key: result.$res_key
                     ),*
                };
                context.json(&response).expect("Temporary for failures, fix this");

                Ok(context)
            }
        }
    };
    (@READ $s:ty, $ctx:ty, $id:ident, {$( $key:ident: $value:ty, )* }, [$( $check:expr, )*]) => {
        paste::paste! {
            #[derive(Debug, serde::Serialize)]
            pub struct [<Read $s Response>] {
                $(
                    $key: $value
                ),*
            }

            #[doc = "Retrieves a `" $s "`."]
            #[thruster::middleware_fn]
            pub async fn [<read_ $s:snake>](mut context: $ctx, _next: MiddlewareNext<$ctx>) -> MiddlewareResult<$ctx> {
                use thruster::context::context_ext::ContextExt;
                let db: &Arc<prisma::PrismaClient> = context.extra.get();
                let db = db.clone();

                let result: prisma::[< $s:snake >]::Data = db.[< $s:snake >]()
                    .find_unique(prisma::[< $s:snake >]::[< $id:snake >]::equals(
                        context.params().get(stringify!([< $id:snake >])).unwrap().param.clone()))
                    .exec()
                    .await
                    .expect("Temporary for failures, fix this")
                    .expect("Wasn't found");

                $(
                    ($check)(&result, &context)?;
                )*

                context.json(&[< Read $s Response >] {
                    $(
                        $key: result.$key
                     ),*
                }).expect("Temporary for failures, fix this");

                Ok(context)
            }
        }
    };
    (@READALL $s:ty, $ctx:ty, $cursor:expr, {$( $key:ident: $value:ty, )* }) => {
        paste::paste! {
            #[derive(Debug, serde::Serialize)]
            pub struct [<ReadAll $s Response>] {
                results: Vec<[< ReadAll $s ElementResponse >]>,
                next: Option<String>
            }

            #[derive(Debug, serde::Serialize)]
            pub struct [<ReadAll $s ElementResponse>] {
                $(
                    $key: $value
                ),*
            }

            #[doc = "Retrieves a `" $s "`."]
            #[thruster::middleware_fn]
            pub async fn [<read_all_ $s:snake>](mut context: $ctx, _next: MiddlewareNext<$ctx>) -> MiddlewareResult<$ctx> {
                use thruster::context::context_ext::ContextExt;
                let db: &Arc<prisma::PrismaClient> = context.extra.get();
                let db = db.clone();

                let results: Vec<prisma::[< $s:snake >]::Data> = db.[< $s:snake >]()
                    .find_many(vec![])
                    .exec()
                    .await
                    .expect("Temporary for failures, fix this");

                context.json(&[< ReadAll $s Response >] {
                    next: results.last().as_ref().map(|&v| ($cursor)(v)),
                    results: results.into_iter().map(|v| [< ReadAll $s ElementResponse >] {
                        $(
                            $key: v.$key
                        ),*
                    }).collect(),
                }).expect("Temporary for failures, fix this");

                Ok(context)
            }
        }
    };
}
