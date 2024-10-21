
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Operation {
    VAdd,
    VMin,
    VMax,
    VScaMul,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum ChipType {
    Opac,
    Scalar,
}