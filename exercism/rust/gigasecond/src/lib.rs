
use time::{Duration, PrimitiveDateTime as DateTime};

// Returns a DateTime one billion seconds after start.
pub fn after(start: DateTime) -> DateTime {
   let added: Option<DateTime> = start.checked_add(Duration::seconds(1000000000));
   match added {
    Some(a) => a,
    None => start
   }
}
