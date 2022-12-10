use contentapi::endpoints::ApiError;
use warp::{reject::Reject, http::uri::InvalidUri};


#[derive(Debug)]
pub struct ErrorWrapper {
    pub error: pages::Error
}

impl Reject for ErrorWrapper {} 
//
////Just a bunch of stupid repetitive stuff because IMO bad design (can't impl Reject on types that aren't defined in the crate)
//impl Reject for ErrorWrapper {}
impl From<ApiError> for ErrorWrapper { fn from(error: ApiError) -> Self { Self { error: pages::Error::Api(error) } } }
impl From<pages::Error> for ErrorWrapper { fn from(error: pages::Error) -> Self { Self { error } } }
//
macro_rules! wrap_from_error {
    ($t:ty) => {
        impl From<$t> for ErrorWrapper { fn from(error: $t) -> Self { Self { error: pages::Error::Other(error.to_string())} }}
    };
}

wrap_from_error!(InvalidUri);
wrap_from_error!(warp::http::Error);
//
////This is so stupid. Oh well
macro_rules! errwrap {
    ($result:expr) => {
        //This is bad, oh well though, maybe I'll fix it later? I assume error mapping only happens
        //ON error, which should rarely happen
        $result.map_err(|error| Into::<ErrorWrapper>::into(error)).map_err(|e| Into::<Rejection>::into(e))
    };
}
pub(crate) use errwrap;
