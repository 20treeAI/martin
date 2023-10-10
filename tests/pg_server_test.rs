use actix_http::Request;
use actix_web::http::StatusCode;
use actix_web::test::{call_and_read_body_json, call_service, read_body, TestRequest};
use ctor::ctor;
use indoc::indoc;
use martin::srv::IndexEntry;
use martin::OneOrMany;
use tilejson::TileJSON;

pub mod utils;
pub use utils::*;

#[ctor]
fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

macro_rules! create_app {
    ($sources:expr) => {{
        let cfg = mock_cfg(indoc::indoc!($sources));
        let sources = mock_sources(cfg).await.0;
        let state = crate::utils::mock_app_data(sources).await;
        ::actix_web::test::init_service(
            ::actix_web::App::new()
                .app_data(state)
                .configure(::martin::srv::router),
        )
        .await
    }};
}

fn test_get(path: &str) -> Request {
    TestRequest::get().uri(path).to_request()
}

#[actix_rt::test]
async fn pg_get_catalog() {
    let app = create_app! { "
postgres:
   connection_string: $DATABASE_URL
"};

    let req = test_get("/catalog");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
    let body = read_body(response).await;
    let sources: Vec<IndexEntry> = serde_json::from_slice(&body).unwrap();

    let expected = "table_source";
    assert_eq!(sources.iter().filter(|v| v.id == expected).count(), 1);

    let expected = "function_zxy_query";
    assert_eq!(sources.iter().filter(|v| v.id == expected).count(), 1);

    let expected = "function_zxy_query_jsonb";
    assert_eq!(sources.iter().filter(|v| v.id == expected).count(), 1);
}

#[actix_rt::test]
async fn pg_get_table_source_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    bad_srid:
      schema: public
      table: table_source
      srid: 3857
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
" };

    let req = test_get("/non_existent");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let req = test_get("/bad_srid");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn pg_get_table_source_rewrite() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
" };

    let req = TestRequest::get()
        .uri("/table_source?token=martin")
        .insert_header(("x-rewrite-url", "/tiles/table_source?token=martin"))
        .to_request();
    let result: TileJSON = call_and_read_body_json(&app, req).await;
    assert_eq!(
        result,
        serde_json::from_str(indoc! {r#"
{
  "name": "table_source",
  "description": "public.table_source.geom",
  "tilejson": "3.0.0",
  "tiles": [
    "http://localhost:8080/tiles/table_source/{z}/{x}/{y}?token=martin"
  ],
  "vector_layers": [
    {
      "id": "table_source",
      "fields": {
        "gid": "int4"
      }
    }
  ],
  "bounds": [-180.0, -90.0, 180.0, 90.0]
}
        "#})
        .unwrap()
    );
}

#[actix_rt::test]
async fn pg_get_table_source_tile_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points_empty_srid:
      schema: public
      table: points_empty_srid
      srid: 900973
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    points3857:
      schema: public
      table: points3857
      srid: 3857
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source_multiple_geom.geom1:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom1
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source_multiple_geom.geom2:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom2
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    MIXPOINTS:
      schema: MIXEDCASE
      table: mixPoints
      srid: 4326
      geometry_column: geoM
      id_column: giD
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        tAble: text
" };

    let req = test_get("/non_existent/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let req = test_get("/table_source/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_table_source_multiple_geom_tile_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source_multiple_geom.geom2:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom2
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    MIXPOINTS:
      schema: MIXEDCASE
      table: mixPoints
      srid: 4326
      geometry_column: geoM
      id_column: giD
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        tAble: text
    points_empty_srid:
      schema: public
      table: points_empty_srid
      srid: 900973
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    points3857:
      schema: public
      table: points3857
      srid: 3857
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source_multiple_geom.geom1:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom1
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
"};

    let req = test_get("/table_source_multiple_geom.geom1/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/table_source_multiple_geom.geom2/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_table_source_tile_minmax_zoom_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    points3857:
      schema: public
      table: points3857
      srid: 3857
      geometry_column: geom
      minzoom: 6
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      minzoom: 6
      maxzoom: 12
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      maxzoom: 6
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
"};
    // zoom = 0 (nothing)
    let req = test_get("/points1/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // zoom = 6 (points1)
    let req = test_get("/points1/6/38/20");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (points1)
    let req = test_get("/points1/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 13 (nothing)
    let req = test_get("/points1/13/4952/2560");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // zoom = 0 (points2)
    let req = test_get("/points2/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 6 (points2)
    let req = test_get("/points2/6/38/20");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (points2)
    let req = test_get("/points2/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 13 (points2)
    let req = test_get("/points2/13/4952/2560");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 0 (nothing)
    let req = test_get("/points3857/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // zoom = 12 (points3857)
    let req = test_get("/points3857/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 0 (table_source)
    let req = test_get("/table_source/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (nothing)
    let req = test_get("/table_source/12/2476/1280");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn pg_get_function_tiles() {
    let app = create_app! { "
postgres:
   connection_string: $DATABASE_URL
"};

    let req = test_get("/function_zoom_xy/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy2/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy_query/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy_query_jsonb/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy_row/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_Mixed_Name/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());

    let req = test_get("/function_zxy_row_key/6/38/20");
    assert!(call_service(&app, req).await.status().is_success());
}

#[actix_rt::test]
async fn pg_get_composite_source_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    table_source_multiple_geom.geom2:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom2
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points_empty_srid:
      schema: public
      table: points_empty_srid
      srid: 900973
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    MIXPOINTS:
      schema: MIXEDCASE
      table: mixPoints
      srid: 4326
      geometry_column: geoM
      id_column: giD
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        tAble: text
    table_source_multiple_geom.geom1:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom1
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points3857:
      schema: public
      table: points3857
      srid: 3857
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
"};
    let req = test_get("/non_existent1,non_existent2");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let req = test_get("/points1,points2,points3857");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_composite_source_tile_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    points_empty_srid:
      schema: public
      table: points_empty_srid
      srid: 900973
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    table_source_multiple_geom.geom1:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom1
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source_multiple_geom.geom2:
      schema: public
      table: table_source_multiple_geom
      srid: 4326
      geometry_column: geom2
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    table_source:
      schema: public
      table: table_source
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: GEOMETRY
      properties:
        gid: int4
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    MIXPOINTS:
      schema: MIXEDCASE
      table: mixPoints
      srid: 4326
      geometry_column: geoM
      id_column: giD
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        tAble: text
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points3857:
      schema: public
      table: points3857
      srid: 3857
      geometry_column: geom
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
"};

    let req = test_get("/non_existent1,non_existent2/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let req = test_get("/points1,points2,points3857/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_composite_source_tile_minmax_zoom_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
  tables:
    points1:
      schema: public
      table: points1
      srid: 4326
      geometry_column: geom
      minzoom: 6
      maxzoom: 13
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
    points2:
      schema: public
      table: points2
      srid: 4326
      geometry_column: geom
      minzoom: 13
      maxzoom: 20
      bounds: [-180.0, -90.0, 180.0, 90.0]
      geometry_type: POINT
      properties:
        gid: int4
"};

    // zoom = 0 (nothing)
    let req = test_get("/points1,points2/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // zoom = 6 (points1)
    let req = test_get("/points1,points2/6/38/20");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (points1)
    let req = test_get("/points1,points2/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 13 (points1, points2)
    let req = test_get("/points1,points2/13/4952/2560");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 14 (points2)
    let req = test_get("/points1,points2/14/9904/5121");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 20 (points2)
    let req = test_get("/points1,points2/20/633856/327787");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 21 (nothing)
    let req = test_get("/points1,points2/21/1267712/655574");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn pg_null_functions() {
    let app = create_app! { "
postgres:
   connection_string: $DATABASE_URL
"};

    let req = test_get("/function_null/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let req = test_get("/function_null_row/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let req = test_get("/function_null_row2/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[actix_rt::test]
async fn pg_get_function_source_ok() {
    let app = create_app! { "
postgres:
   connection_string: $DATABASE_URL
"};

    let req = test_get("/non_existent");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let req = test_get("/function_zoom_xy");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy_query");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy_query_jsonb");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy_query_test");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy_row");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_Mixed_Name");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    let req = test_get("/function_zxy_row_key");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_function_source_ok_rewrite() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
"};

    let req = TestRequest::get()
        .uri("/function_zxy_query?token=martin")
        .insert_header(("x-rewrite-url", "/tiles/function_zxy_query?token=martin"))
        .to_request();
    let result: TileJSON = call_and_read_body_json(&app, req).await;
    assert_eq!(
        result.tiles,
        &["http://localhost:8080/tiles/function_zxy_query/{z}/{x}/{y}?token=martin"]
    );

    let req = TestRequest::get()
        .uri("/function_zxy_query_jsonb?token=martin")
        .insert_header((
            "x-rewrite-url",
            "/tiles/function_zxy_query_jsonb?token=martin",
        ))
        .to_request();
    let result: TileJSON = call_and_read_body_json(&app, req).await;
    assert_eq!(
        result.tiles,
        &["http://localhost:8080/tiles/function_zxy_query_jsonb/{z}/{x}/{y}?token=martin"]
    );
}

#[actix_rt::test]
async fn pg_get_function_source_tile_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
"};

    let req = test_get("/function_zxy_query/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_function_source_tile_minmax_zoom_ok() {
    let app = create_app! {"
postgres:
  connection_string: $DATABASE_URL
  functions:
    function_source1:
      schema: public
      function: function_zxy_query
    function_source2:
      schema: public
      function: function_zxy_query
      minzoom: 6
      maxzoom: 12
      bounds: [-180.0, -90.0, 180.0, 90.0]
"};

    // zoom = 0 (function_source1)
    let req = test_get("/function_source1/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 6 (function_source1)
    let req = test_get("/function_source1/6/38/20");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (function_source1)
    let req = test_get("/function_source1/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 13 (function_source1)
    let req = test_get("/function_source1/13/4952/2560");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 0 (nothing)
    let req = test_get("/function_source2/0/0/0");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // zoom = 6 (function_source2)
    let req = test_get("/function_source2/6/38/20");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 12 (function_source2)
    let req = test_get("/function_source2/12/2476/1280");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());

    // zoom = 13 (nothing)
    let req = test_get("/function_source2/13/4952/2560");
    let response = call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn pg_get_function_source_query_params_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
"};

    let req = test_get("/function_zxy_query_test/0/0/0");
    let response = call_service(&app, req).await;
    assert!(response.status().is_server_error());

    let req = test_get("/function_zxy_query_test/0/0/0?token=martin");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_get_health_returns_ok() {
    let app = create_app! { "
postgres:
  connection_string: $DATABASE_URL
"};

    let req = test_get("/health");
    let response = call_service(&app, req).await;
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn pg_tables_feature_id() {
    let cfg = mock_pgcfg(indoc! {"
connection_string: $DATABASE_URL
tables:
  id_and_prop:
    schema: MIXEDCASE
    table: mixPoints
    srid: 4326
    geometry_column: geoM
    id_column: giD
    bounds: [-180.0, -90.0, 180.0, 90.0]
    geometry_type: POINT
    properties:
      TABLE: text
      giD: int4
  no_id:
    schema: MIXEDCASE
    table: mixPoints
    srid: 4326
    geometry_column: geoM
    bounds: [-180.0, -90.0, 180.0, 90.0]
    geometry_type: POINT
    properties:
      TABLE: text
  id_only:
    schema: MIXEDCASE
    table: mixPoints
    srid: 4326
    geometry_column: geoM
    id_column: giD
    bounds: [-180.0, -90.0, 180.0, 90.0]
    geometry_type: POINT
    properties:
      TABLE: text
  prop_only:
    schema: MIXEDCASE
    table: mixPoints
    srid: 4326
    geometry_column: geoM
    bounds: [-180.0, -90.0, 180.0, 90.0]
    geometry_type: POINT
    properties:
      giD: int4
      TABLE: text
"});
    let mock = mock_sources(cfg.clone()).await;

    let src = table(&mock, "no_id");
    assert_eq!(src.id_column, None);
    assert!(matches!(&src.properties, Some(v) if v.len() == 1));
    assert_eq!(
        source(&mock, "no_id").get_tilejson(),
        serde_json::from_str(indoc! {r#"
{
  "name": "no_id",
  "description": "MixedCase.MixPoints.Geom",
  "tilejson": "3.0.0",
  "tiles": [],
  "vector_layers": [
    {
      "id": "no_id",
      "fields": {"TABLE": "text"}
    }
  ],
  "bounds": [-180.0, -90.0, 180.0, 90.0]
}
        "#})
        .unwrap()
    );

    let src = table(&mock, "id_only");
    assert_eq!(src.id_column, some("giD"));
    assert!(matches!(&src.properties, Some(v) if v.len() == 1));

    let src = table(&mock, "id_and_prop");
    assert_eq!(src.id_column, some("giD"));
    assert!(matches!(&src.properties, Some(v) if v.len() == 2));

    let src = table(&mock, "prop_only");
    assert_eq!(src.id_column, None);
    assert!(matches!(&src.properties, Some(v) if v.len() == 2));

    // --------------------------------------------

    let state = mock_app_data(mock.0).await;
    let app = ::actix_web::test::init_service(
        ::actix_web::App::new()
            .app_data(state)
            .configure(::martin::srv::router),
    )
    .await;

    let OneOrMany::One(cfg) = cfg.postgres.unwrap() else {
        panic!()
    };
    for (name, _) in cfg.tables.unwrap_or_default() {
        let req = test_get(format!("/{name}/0/0/0").as_str());
        let response = call_service(&app, req).await;
        assert!(response.status().is_success());
    }
}
