use diesel::prelude::*;
use turbo_diesel::prelude::*;

diesel::table! {
  users (id) {
      id -> Varchar,
      name -> Varchar,
  }
}

#[derive(Clone, Debug, Insertable, Queryable, Identifiable)]
#[diesel(primary_key(id))]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbUser {
  pub id: String,
  pub name: String,
}

impl DbModelCreate for DbUser {}
impl DbModelDelByPk for DbUser {}
impl DbModelDelBy for DbUser {
  fn gen_del_query<D>(
    filter: &GenericFilter,
  ) -> diesel::query_builder::BoxedDeleteStatement<
    'static,
    <D as Connection>::Backend,
    <Self as diesel::associations::HasTable>::Table,
  >
  where
    D: diesel::r2d2::R2D2Connection
      + Connection
      + diesel::connection::LoadConnection
      + 'static,
    Self: diesel::associations::HasTable,
  {
    let mut query =
      diesel::delete(<Self as diesel::associations::HasTable>::table())
        .into_boxed();
    let r#where = filter.r#where.clone().unwrap_or_default();

    query
  }
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
  let db = DbDriver::<SqliteConnection>::new("file:///tmp/test.db")?;
  db.create(&DbUser {
    id: "1".to_string(),
    name: "test".to_string(),
  })
  .await
  .unwrap();
  db.del_by_pk::<DbUser, _>("123").await.unwrap();
  let filter =
    GenericFilter::new().r#where("id", GenericClause::Eq("1".to_owned()));
  db.del_by::<DbUser>(&filter).await.unwrap();
  let db = DbDriver::<PgConnection>::new("file:///tmp/test.db")?;
  db.create(&DbUser {
    id: "1".to_string(),
    name: "test".to_string(),
  })
  .await
  .unwrap();
  db.del_by_pk::<DbUser, _>("123").await.unwrap();
  let filter =
    GenericFilter::new().r#where("id", GenericClause::Eq("1".to_owned()));
  db.del_by::<DbUser>(&filter).await.unwrap();
  Ok(())
}
