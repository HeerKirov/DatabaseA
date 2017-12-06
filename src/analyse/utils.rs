pub trait CharUtils {
    fn is_signal(&self, signals:&[char]) -> bool;
    fn is_std_signal(&self) -> bool;
    fn is_std_else(&self, elses:&[char]) -> bool;
    fn is_alphas(&self) -> bool;
    fn is_digits(&self) -> bool;
    fn is_space(&self) -> bool;
    fn is_enter(&self) -> bool;
    fn is_end(&self) -> bool;
}
impl CharUtils for char {
    fn is_signal(&self, signals:&[char]) -> bool {
        for i in 0..signals.len() {
            if *self == signals[i] {
                return true;
            }
        }
        false
    }
    fn is_std_signal(&self) -> bool {
        let std_signal = [',', '.', '(', ')', '\"', ';', '=', '!', '<', '>', '+', '-', '*', '/', '^'];
        self.is_signal(&std_signal)
    }
    fn is_std_else(&self, elses:&[char]) -> bool {
        for i in 0..elses.len() {
            if *self == elses[i] { return false; }
        }
        self.is_std_signal()
    }
    fn is_alphas(&self) -> bool {
        (*self >= 'a' && *self <= 'z') || (*self >= 'A' && *self <= 'Z')
    }
    fn is_digits(&self) -> bool {
        *self >= '0' && *self <= '9'
    }
    fn is_space(&self) -> bool {
        *self == ' ' || *self == '\t'
    }
    fn is_enter(&self) -> bool {
        *self == '\n' || *self == '\r'
    }
    fn is_end(&self) -> bool {
        false // todo END
    }
}