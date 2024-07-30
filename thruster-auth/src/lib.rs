use async_trait::async_trait;
use deadpool_postgres::GenericClient;
use thruster::{
    errors::{ErrorSet, ThrusterError},
    middleware::cookies::HasCookies,
    Context, MiddlewareNext, MiddlewareResult,
};
use tokio_postgres::types::ToSql;

pub const COOKIE_AUTH_TOKEN: &str = "auth-token";
const COOKIE_AUTH_KEY: &str = "auth-token=";

#[async_trait]
pub trait UserTrait: Sized {
    type ID: ToSql + Sync;
    type Error;
    type NewUser;

    async fn fetch<DB: GenericClient + Send + Sync>(
        db: &mut DB,
        id: &Self::ID,
    ) -> Result<Self, Self::Error>;
    async fn create<DB: GenericClient + Send + Sync>(
        db: &mut DB,
        t: Self::NewUser,
    ) -> Result<Self, Self::Error>;
}

#[async_trait]
pub trait SessionTrait: Sized {
    type UserID;
    type Error;

    async fn fetch<DB: GenericClient + Send + Sync>(
        db: &mut DB,
        token: &str,
    ) -> Result<Self, Self::Error>;
    async fn create<DB: GenericClient + Send + Sync>(
        db: &mut DB,
        user_id: &Self::UserID,
    ) -> Result<Self, Self::Error>;
    fn user_id(&self) -> &Self::UserID;
}

pub trait SetAuthenticatedUser {
    type User;

    fn set_authenticated_user(&mut self, user: Self::User);
}

pub trait GetDb {
    type Db: GenericClient + Send + Sync;

    fn get_db(&self) -> Self::Db;
}

pub async fn authenticate_cookie<
    U: UserTrait<ID = UserId>,
    S: SessionTrait<UserID = UserId>,
    UserId,
    Db: GenericClient + Send + Sync,
    T: Context + Default + HasCookies + GetDb<Db = Db> + SetAuthenticatedUser<User = U>,
>(
    mut ctx: T,
    next: MiddlewareNext<T>,
) -> MiddlewareResult<T> {
    let token = ctx.get_cookies().iter().find_map(|v| {
        if v.starts_with(COOKIE_AUTH_KEY) {
            Some(v.replace(COOKIE_AUTH_KEY, ""))
        } else {
            None
        }
    });

    match token {
        Some(token) => {
            let mut db = ctx.get_db();
            let session = S::fetch(&mut db, &token)
                .await
                .map_err(|_| ThrusterError::unauthorized_error(T::default()))?;
            let user = U::fetch(&mut db, session.user_id())
                .await
                .map_err(|_| ThrusterError::unauthorized_error(T::default()))?;

            ctx.set_authenticated_user(user);
        }
        None => {
            return Err(ThrusterError::unauthorized_error(ctx));
        }
    }

    next(ctx).await
}

// #[cfg(all(models, routes))]
mod routes {
    use crate::{GetDb, SessionTrait, SetAuthenticatedUser, UserTrait};
    use deadpool_postgres::GenericClient;
    use thruster::{middleware::cookies::HasCookies, Context, MiddlewareNext, MiddlewareResult};

    pub async fn create_session<
        U: UserTrait<ID = UserId>,
        S: SessionTrait<UserID = UserId>,
        UserId,
        Db: GenericClient + Send + Sync,
        T: Context + Default + HasCookies + GetDb<Db = Db> + SetAuthenticatedUser<User = U>,
    >(
        mut ctx: T,
        _next: MiddlewareNext<T>,
    ) -> MiddlewareResult<T> {
        Ok(ctx)
    }

    pub async fn create_user<
        U: UserTrait<ID = UserId>,
        S: SessionTrait<UserID = UserId>,
        UserId,
        Db: GenericClient + Send + Sync,
        T: Context + Default + HasCookies + GetDb<Db = Db> + SetAuthenticatedUser<User = U>,
    >(
        mut ctx: T,
        _next: MiddlewareNext<T>,
    ) -> MiddlewareResult<T> {
        Ok(ctx)
    }
}

// #[cfg(models)]
mod models {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use deadpool_postgres::GenericClient;
    use usual::{base::Model, base::TryGetRow, query, UsualModel};
    use uuid::Uuid;

    use crate::{SessionTrait, UserTrait};

    #[derive(Clone, Debug, UsualModel)]
    pub struct User {
        id: Uuid,
        email: String,
        updated_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    }

    pub struct CreateUser {
        email: String,
    }

    #[async_trait]
    impl UserTrait for User {
        type ID = Uuid;
        type Error = tokio_postgres::Error;
        type NewUser = CreateUser;

        async fn fetch<DB: GenericClient + Send + Sync>(
            db: &mut DB,
            id: &Self::ID,
        ) -> Result<Self, Self::Error> {
            Ok(User::from_row(
                &db.query_one(
                    query!("SELECT {User} FROM users WHERE id = $1").as_str(),
                    &[&id],
                )
                .await?,
            ))
        }

        async fn create<DB: GenericClient + Send + Sync>(
            db: &mut DB,
            t: Self::NewUser,
        ) -> Result<Self, Self::Error> {
            Ok(User::from_row(
                &db.query_one(
                    query!("INSERT INTO users (email) VALUES ($1) RETURNING {User}").as_str(),
                    &[&t.email],
                )
                .await?,
            ))
        }
    }

    #[derive(Clone, Debug, UsualModel)]
    pub struct Session {
        user_id: Uuid,
        token: String,
        updated_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    }

    #[async_trait]
    impl SessionTrait for Session {
        type UserID = Uuid;
        type Error = tokio_postgres::Error;

        async fn fetch<DB: GenericClient + Send + Sync>(
            db: &mut DB,
            token: &str,
        ) -> Result<Self, Self::Error> {
            Ok(Session::from_row(
                &db.query_one(
                    query!("SELECT {Session} FROM sessions WHERE token = $1").as_str(),
                    &[&token],
                )
                .await?,
            ))
        }

        async fn create<DB: GenericClient + Send + Sync>(
            db: &mut DB,
            user_id: &Self::UserID,
        ) -> Result<Self, Self::Error> {
            Ok(Session::from_row(
                &db.query_one(
                    query!(
                        "INSERT INTO sessions (token, user_id) VALUES ($1, $2) RETURNING {Session}"
                    )
                    .as_str(),
                    &[&"", &user_id],
                )
                .await?,
            ))
        }

        fn user_id(&self) -> &Self::UserID {
            &self.user_id
        }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
