use time::PreciseTime;

// Do not name one of these `_`, or it will be dropped immediately
pub struct ScopeTimer{
    // Logs time between creation to when `drop`ped.
    name:     String,
    creation: PreciseTime
}

impl ScopeTimer{
    pub fn new(name: &str) -> ScopeTimer{
        ScopeTimer{
            name:     String::from(name),
            creation: PreciseTime::now()
        }
    }
}

impl Drop for ScopeTimer{
    fn drop(&mut self){
        println!("{} took {}ns",
                 self.name,
                 match self.creation.to(PreciseTime::now()).num_nanoseconds(){
                     Some(x) => x,
                     None    => -1 // ehhh
                 });
    }
}
