
// use warp::Filter;
// use cookie::{Cookie, CookieJar};
// use serde::Deserialize;
// use std::collections::HashMap;
// use std::sync::{Arc, Mutex};
// use warp::Rejection;

// // Simple in-memory storage for user sessions
// pub type Sessions = Arc<Mutex<HashMap<String, String>>>;

// #[derive(Deserialize)]
// pub struct LoginRequest {
//     username: String,
//     password: String,
// }


// // Middleware to check if a user is logged in
// pub async fn verify_session(sessions: Sessions, cookie_header: Option<String>) -> Result<(), Rejection> {
//     let cookie_header = cookie_header.unwrap().clone();
    
//         let mut cookie_jar = CookieJar::new();
//         let parts: Vec<String> = cookie_header.split(' ').map(|s| s.to_string()).collect();      //  let parts: Vec<String> = cookie_header.split(' ');//.map(|s| s.to_string()).collect();
//         for cookie in parts {
//             let cookie: String = cookie.clone().trim().to_string();
//             if let Ok(cookie) = Cookie::parse(cookie) {
//                 cookie_jar.add_original(cookie.clone());
//             }
//         }
    

//         if let Some(session_cookie) = cookie_jar.get("session") {
//             let session_id = session_cookie.value();
//             let sessions = sessions.lock().unwrap();
//             if sessions.contains_key(session_id) {
//                 return Ok(());
//             }
//         }
   
//  //  let s = println!("{:?}",cookie_header);

//     // Redirect to login page if session is invalid
//     Err(warp::reject::not_found())

// }
