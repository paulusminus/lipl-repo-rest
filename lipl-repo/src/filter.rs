use std::sync::{Arc, RwLock};
use warp::{body, path, Filter};
use warp::filters::query;
use lipl_io::model::{Db};
use crate::constant::{API, VERSION};
use crate::handler::lyric as lyric_handler;
use crate::handler::playlist as playlist_handler;

macro_rules! join_paths {
    ($head:expr, $($rest:expr),*) => { warp::path($head)$(.and(warp::path($rest)))* };
}

macro_rules! or {
    ($head:expr, $($rest:expr),*) => { $head$(.or($rest))* };
}

macro_rules! and {
    ($head:expr, $($rest:expr),*) => { $head$(.and($rest))* };
}

macro_rules! create_fn {
    ($name:ident, $handler:ident) => {
        pub fn $name(db: Arc<RwLock<Db>>, name: &'static str) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        {
            let db_filter  = warp::any().map(move || db.clone());
            let prefix = join_paths!(API, VERSION, name);
        
            let list         = and! (warp::get()   , prefix, path::end()  , db_filter.clone(), query::query()   ) .and_then($handler::list);
            let summaries    = and! (warp::get()   , prefix, path::end()  , db_filter.clone()                   ) .and_then($handler::list_summary);
            let item         = and! (warp::get()   , prefix, path::param(), db_filter.clone()                   ) .and_then($handler::item);
            let post         = and! (warp::post()  , prefix, path::end()  , db_filter.clone(), body::json()     ) .and_then($handler::post);
            let put          = and! (warp::put()   , prefix, path::param(), db_filter.clone(), body::json()     ) .and_then($handler::put);
            let delete       = and! (warp::delete(), prefix, path::param(), db_filter.clone()                   ) .and_then($handler::delete);
        
            or!(list, summaries, item, post, put, delete)
        }
    };
}

create_fn!(get_lyric_routes, lyric_handler);
create_fn!(get_playlist_routes, playlist_handler);