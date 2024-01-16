use std::future::Future;

use diesel::{
  prelude::*,
  associations::HasTable,
  r2d2::{ConnectionManager, Pool, PooledConnection},
  query_dsl, query_builder,
  connection::LoadConnection,
};

use crate::prelude::*;

/// A Database driver.
/// Will get a connection from the pool and execute queries.
/// Support for multiple database types is provided by the `diesel` crate.
///
pub struct DbDriver<D>
where
  D: diesel::r2d2::R2D2Connection + 'static,
{
  pool: Pool<ConnectionManager<D>>,
}

/// Implement `Clone` for `DbDriver`.
impl<D> Clone for DbDriver<D>
where
  D: diesel::r2d2::R2D2Connection + 'static,
{
  fn clone(&self) -> Self {
    Self {
      pool: self.pool.clone(),
    }
  }
}

impl<D> DbDriver<D>
where
  D: diesel::r2d2::R2D2Connection + Connection + LoadConnection + 'static,
{
  /// Create a new database driver.
  pub fn new(db_url: &str) -> Result<Self, std::io::Error> {
    let manager = ConnectionManager::<D>::new(db_url);
    let pool = Pool::builder().build(manager).map_err(|err| {
      std::io::Error::new(std::io::ErrorKind::NotConnected, err)
    })?;
    Ok(Self { pool })
  }

  /// Get a connection from the pool.
  pub fn get_conn(
    &self,
  ) -> Result<PooledConnection<ConnectionManager<D>>, std::io::Error>
where {
    self.pool.get().map_err(|err| {
      std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        format!("Failed to get connection: {}", err),
      )
    })
  }

  /// Execute a function with a connection from the pool.
  pub async fn execute<F, R>(&self, f: F) -> Result<R, diesel::result::Error>
  where
    F: FnOnce(
        PooledConnection<ConnectionManager<D>>,
      ) -> Result<R, diesel::result::Error>
      + Send
      + 'static,
    R: Send + 'static,
  {
    let self_ptr = self.clone();
    ntex::rt::spawn_blocking(move || {
      let conn = self_ptr
        .get_conn()
        .map_err(|_| diesel::result::Error::BrokenTransactionManager)?;
      f(conn)
    })
    .await
    .map_err(|_| diesel::result::Error::BrokenTransactionManager)?
  }

  /// Handle the DbModelCreate
  pub async fn create<I>(&self, item: &I) -> Result<I, diesel::result::Error>
  where
    I: DbModelCreate + Send + Clone + Sync + 'static,
    I: HasTable + diesel::Insertable<I::Table>,
    I::Table: HasTable<Table = I::Table> + diesel::Table,
    diesel::query_builder::InsertStatement<
      I::Table,
      <I as diesel::Insertable<I::Table>>::Values,
    >: diesel::query_dsl::LoadQuery<'static, D, I>,
  {
    I::create(self, item).await
  }

  pub async fn del_by_pk<I, Pk>(&self, pk: &Pk) -> Result<(), diesel::result::Error>
  where
    Pk: Sync + ToOwned + std::fmt::Display + ?Sized,
    <Pk as ToOwned>::Owned: Send + 'static,
    I: Sized + HasTable + DbModelDelByPk,
    I::Table: query_dsl::methods::FindDsl<<Pk as ToOwned>::Owned> + HasTable<Table = I::Table>,
    diesel::helper_types::Find<I::Table, <Pk as ToOwned>::Owned>: query_builder::IntoUpdateTarget,
    query_builder::DeleteStatement<
      <diesel::helper_types::Find<I::Table, <Pk as ToOwned>::Owned> as HasTable>::Table,
      <diesel::helper_types::Find<I::Table, <Pk as ToOwned>::Owned> as query_builder::IntoUpdateTarget>::WhereClause,
    >: query_builder::QueryFragment<<D as Connection>::Backend> + query_builder::QueryId,
  {
    I::del_by_pk(self, pk).await
  }

  pub async fn del_by<I>(
    &self,
    filter: &GenericFilter,
  ) -> Result<(), diesel::result::Error>
  where
    I: Sized + HasTable + DbModelDelBy,
    <I as HasTable>::Table: query_builder::QueryId + 'static,
    <<I as HasTable>::Table as diesel::QuerySource>::FromClause:
      diesel::query_builder::QueryFragment<<D as Connection>::Backend>,
    <<I as HasTable>::Table as diesel::QuerySource>::FromClause:
      diesel::query_builder::QueryFragment<<D as Connection>::Backend>,
    <D as diesel::Connection>::Backend:
      diesel::internal::derives::multiconnection::DieselReserveSpecialization,
  {
    I::del_by(self, filter).await
  }
}

pub trait DbModelCreate {
  fn create<D>(
    db: &DbDriver<D>,
    item: &Self,
  ) -> impl Future<Output = Result<Self, diesel::result::Error>> + Send
  where
    D: diesel::r2d2::R2D2Connection
      + diesel::connection::LoadConnection
      + 'static,
    Self: Sized
      + Send
      + Clone
      + Sync
      + HasTable
      + diesel::Insertable<Self::Table>
      + 'static,
    Self::Table: diesel::Table,
    query_builder::InsertStatement<
      Self::Table,
      <Self as diesel::Insertable<Self::Table>>::Values,
    >: query_dsl::LoadQuery<'static, D, Self>,
  {
    async {
      let item = item.to_owned();
      db.execute(move |mut conn| {
        let item = diesel::insert_into(<Self as HasTable>::table())
          .values(item)
          .get_result(&mut conn)?;
        Ok::<_, diesel::result::Error>(item)
      })
      .await
    }
  }
}

pub trait DbModelDelByPk {
  fn del_by_pk<D, Pk>(
    db: &DbDriver<D>,
    pk: &Pk,
  ) -> impl Future<Output = Result<(), diesel::result::Error>> + Send
  where
  D: diesel::r2d2::R2D2Connection
  + Connection
  + LoadConnection
  + 'static,
  Pk: Sync + ToOwned + std::fmt::Display + ?Sized,
  <Pk as ToOwned>::Owned: Send + 'static,
  Self: Sized + HasTable,
  Self::Table: query_dsl::methods::FindDsl<<Pk as ToOwned>::Owned> + HasTable<Table = Self::Table>,
  diesel::helper_types::Find<Self::Table, <Pk as ToOwned>::Owned>: query_builder::IntoUpdateTarget,
  query_builder::DeleteStatement<
    <diesel::helper_types::Find<Self::Table, <Pk as ToOwned>::Owned> as HasTable>::Table,
    <diesel::helper_types::Find<Self::Table, <Pk as ToOwned>::Owned> as query_builder::IntoUpdateTarget>::WhereClause,
  >: query_builder::QueryFragment<<D as Connection>::Backend> + query_builder::QueryId,
  {
    async {
      let pk = pk.to_owned();
      db.execute(move |mut conn| {
        diesel::delete(<Self::Table as HasTable>::table().find(pk))
          .execute(&mut conn)?;
        Ok::<_, diesel::result::Error>(())
      })
      .await
    }
  }
}

pub trait DbModelDelBy {
  fn gen_del_query<D>(
    filter: &GenericFilter,
  ) -> diesel::query_builder::BoxedDeleteStatement<
    'static,
    <D as Connection>::Backend,
    <Self as diesel::associations::HasTable>::Table,
  >
  where
    D: diesel::r2d2::R2D2Connection + Connection + LoadConnection + 'static,
    Self: diesel::associations::HasTable;

  fn del_by<D>(
    db: &DbDriver<D>,
    filter: &GenericFilter,
  ) -> impl std::future::Future<Output = Result<(), diesel::result::Error>> + Send
  where
    D: diesel::r2d2::R2D2Connection + Connection + LoadConnection + 'static,
    Self: Sized + HasTable,
    <Self as HasTable>::Table: query_builder::QueryId + 'static,
    <<Self as HasTable>::Table as diesel::QuerySource>::FromClause:
      diesel::query_builder::QueryFragment<<D as Connection>::Backend>,
    <<Self as HasTable>::Table as diesel::QuerySource>::FromClause:
      diesel::query_builder::QueryFragment<<D as Connection>::Backend>,
    <D as diesel::Connection>::Backend:
      diesel::internal::derives::multiconnection::DieselReserveSpecialization,
  {
    async {
      let filter = filter.clone();
      db.execute(move |mut conn| {
        let query = Self::gen_del_query::<D>(&filter);
        query.execute(&mut conn)?;
        Ok::<_, diesel::result::Error>(())
      })
      .await
    }
  }
}
