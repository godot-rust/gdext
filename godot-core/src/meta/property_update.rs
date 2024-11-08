pub struct PropertyUpdate<'a, C, T> {
    pub new_value: T,
    pub field_name: &'a str, // might also be &'a StringName, depending on what's available
    pub get_field_mut: fn(&mut C) -> &mut T,
}

impl<C, T> PropertyUpdate<'_, C, T> {
    pub fn set(self, obj: &mut C) {
        *(self.get_field_mut)(obj) = self.new_value;
    }
    pub fn set_custom(self, obj: &mut C, value: T) {
        *(self.get_field_mut)(obj) = value;
    }
}
