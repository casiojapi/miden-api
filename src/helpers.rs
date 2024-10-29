#[cfg(feature = "debug")]
#[macro_export]
macro_rules! stdpr {
    ( $x:expr ) => {
        let _o = $x;
        println!("status: {}", _o.status);
        println!("stdout {:?}", String::from_utf8_lossy(&_o.stdout));
        println!("stderr {:?}", String::from_utf8_lossy(&_o.stderr));
    };
}


#[macro_export]
macro_rules! plain_enum {
    ( $x:expr ) => {
    };
}
