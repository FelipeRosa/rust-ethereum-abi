#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Uint(usize),
    Int(usize),
    Address,
    Bool,
    FixedBytes(usize),
    FixedArray(Box<Type>, usize),
    String,
    Bytes,
    Array(Box<Type>),
    Tuple(Vec<(String, Type)>),
}

impl Type {
    pub fn is_dynamic(&self) -> bool {
        match self {
            Type::Uint(_) => false,
            Type::Int(_) => false,
            Type::Address => false,
            Type::Bool => false,
            Type::FixedBytes(_) => false,
            Type::FixedArray(ty, _) => ty.is_dynamic(),
            Type::String => true,
            Type::Bytes => true,
            Type::Array(_) => true,
            Type::Tuple(tys) => tys.iter().any(|(_, ty)| ty.is_dynamic()),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Uint(size) => write!(f, "uint{}", size),
            Type::Int(size) => write!(f, "int{}", size),
            Type::Address => write!(f, "address"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::FixedBytes(size) => write!(f, "bytes{}", size),
            Type::Bytes => write!(f, "bytes"),
            Type::FixedArray(ty, size) => write!(f, "{}[{}]", ty, size),
            Type::Array(ty) => write!(f, "{}[]", ty),
            Type::Tuple(_) => todo!(),
        }
    }
}
