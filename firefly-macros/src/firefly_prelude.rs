use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

#[derive(Debug, FromMeta)]
struct MacroArgs {
    context_type: Ident,
}

pub fn prelude(args: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };
    let args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let ctx = args.context_type;

    quote! {
    // START PRELUDE
    use std::str::FromStr;

    // INTO INNER
    pub trait IntoInner<T> {
        fn into_inner(self) -> T;
    }

    impl<O: HasId, T: HasOwnerTrait<T>> IntoInner<T> for HasOwner<O, T> {
        fn into_inner(self) -> T {
            self.1
        }
    }

    impl<T: FromRouteTrait> IntoInner<T> for FromRoute<T> {
        fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T: FromContextTrait> IntoInner<T> for FromContext<T> {
        fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T: FromJsonTrait> IntoInner<T> for FromJson<T> {
        fn into_inner(self) -> T {
            self.0
        }
    }

    // HAS ID
    pub struct HasOwner<O: HasId, T: HasOwnerTrait<T>>(O, T);
    pub trait HasId
    where
        Self: Sized,
    {
        fn get_id(&self) -> &uuid::Uuid;
    }
    impl<T> HasId for FromRoute<T>
    where
        T: HasId + FromRouteTrait,
    {
        fn get_id(&self) -> &uuid::Uuid {
            self.0.get_id()
        }
    }
    impl<T> HasId for FromContext<T>
    where
        T: HasId + FromContextTrait,
    {
        fn get_id(&self) -> &uuid::Uuid {
            self.0.get_id()
        }
    }

    // HAS OWNER
    #[async_trait::async_trait]
    pub trait HasOwnerTrait<T>
    where
        Self: Sized,
    {
        async fn check_owner(
            self,
            owner: &(impl HasId + Send + Sync),
        ) -> Result<T, Box<dyn std::error::Error>>;
    }
    #[async_trait::async_trait]
    impl<T> HasOwnerTrait<T> for FromRoute<T>
    where
        T: 'static + HasOwnerTrait<T> + FromRouteTrait + Send,
    {
        async fn check_owner(
            self,
            owner: &(impl HasId + Send + Sync),
        ) -> Result<T, Box<dyn std::error::Error>> {
            self.0.check_owner(owner).await
        }
    }
    #[async_trait::async_trait]
    impl<T> HasOwnerTrait<FromRoute<T>> for FromRoute<T>
    where
        T: 'static + HasOwnerTrait<T> + FromRouteTrait + Send,
    {
        async fn check_owner(
            self,
            owner: &(impl HasId + Send + Sync),
        ) -> Result<FromRoute<T>, Box<dyn std::error::Error>> {
            self.check_owner(owner).await
        }
    }
    #[async_trait::async_trait]
    impl<T> HasOwnerTrait<T> for FromContext<T>
    where
        T: 'static + HasOwnerTrait<T> + FromContextTrait + Send,
    {
        async fn check_owner(
            self,
            owner: &(impl HasId + Send + Sync),
        ) -> Result<T, Box<dyn std::error::Error>> {
            self.0.check_owner(owner).await
        }
    }
    #[async_trait::async_trait]
    impl<T> HasOwnerTrait<FromContext<T>> for FromContext<T>
    where
        T: 'static + HasOwnerTrait<T> + FromContextTrait + Send,
    {
        async fn check_owner(
            self,
            owner: &(impl HasId + Send + Sync),
        ) -> Result<FromContext<T>, Box<dyn std::error::Error>> {
            self.check_owner(owner).await
        }
    }

    // FROM ROUTE
    pub struct FromRoute<T: FromRouteTrait>(T);
    #[async_trait::async_trait]
    pub trait FromRouteTrait
    where
        Self: Sized,
    {
        async fn from_route(ctx: &#ctx) -> Result<Self, Box<dyn std::error::Error>>;
    }

    // FROM CONTEXT
    pub struct FromContext<T: FromContextTrait>(T);
    #[async_trait::async_trait]
    pub trait FromContextTrait
    where
        Self: Sized,
    {
        async fn from_context(ctx: &#ctx) -> Result<Self, Box<dyn std::error::Error>>;
    }

    // FROM JSON
    pub struct FromJson<T: FromJsonTrait>(T);
    #[async_trait::async_trait]
    pub trait FromJsonTrait
    where
        Self: Sized,
    {
        async fn from_json(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>>;
    }

    #[async_trait::async_trait]
    impl<T> FromJsonTrait for T
    where
        T: serde::de::DeserializeOwned,
    {
        async fn from_json(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>> {
            use thruster::context::context_ext::ContextExt;
            let v: T = ctx.get_json().await?;

            Ok(v)
        }
    }

    // EXTRACTOR
    pub trait Extractor<T>
    where
        Self: Sized,
        T: Sized,
    {
        async fn extract(ctx: &mut #ctx) -> Result<T, Box<dyn std::error::Error>>;
    }
    impl<T> Extractor<FromRoute<T>> for FromRoute<T>
    where
        T: FromRouteTrait,
    {
        async fn extract(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(FromRoute(T::from_route(ctx).await?))
        }
    }
    impl<T> Extractor<FromContext<T>> for FromContext<T>
    where
        T: FromContextTrait,
    {
        async fn extract(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(FromContext(T::from_context(ctx).await?))
        }
    }
    impl<O, T> Extractor<HasOwner<O, T>> for HasOwner<O, T>
    where
        O: 'static + Extractor<O> + HasId + Send + Sync,
        T: Extractor<T> + HasOwnerTrait<T> + Into<T>,
    {
        async fn extract(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>> {
            let o = O::extract(ctx).await?;
            let t = T::extract(ctx).await?;
            let t = t.check_owner(&o).await?;

            Ok(HasOwner(o, t))
        }
    }
    impl<T> Extractor<FromJson<T>> for FromJson<T>
    where
        T: FromJsonTrait,
    {
        async fn extract(ctx: &mut #ctx) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(FromJson(T::from_json(ctx).await?))
        }
    }
    // END PRELUDE
    }
    .into()
}
