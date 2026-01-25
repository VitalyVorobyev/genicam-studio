#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModel {
    pub name: String,
}

impl DeviceModel {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceModel;

    #[test]
    fn creates_model() {
        let model = DeviceModel::new("Camera-1");
        assert_eq!(model.name, "Camera-1");
    }
}
