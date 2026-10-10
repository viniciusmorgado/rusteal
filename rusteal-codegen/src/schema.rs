#![allow(dead_code)]

use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct ClassesFile {
    pub classes: Vec<ClassInfo>,
}

#[derive(Deserialize)]
pub struct StructsFile {
    pub structs: Vec<StructInfo>,
}

#[derive(Deserialize)]
pub struct EnumsFile {
    pub enums: Vec<EnumInfo>,
}

#[derive(Deserialize, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub cpp_name: String,
    pub package: String,
    pub header: String,
    #[serde(default = "default_true")]
    pub header_public: bool,
    #[serde(deserialize_with = "deser_flags_u32")]
    pub class_flags: u32,
    #[serde(rename = "super")]
    pub super_class: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub props: Vec<PropertyInfo>,
    #[serde(default)]
    pub funcs: Vec<FunctionInfo>,
}

#[derive(Deserialize, Clone)]
pub struct StructInfo {
    pub name: String,
    pub cpp_name: String,
    pub package: String,
    #[serde(default)]
    pub header: String,
    #[serde(default = "default_true")]
    pub header_public: bool,
    #[serde(deserialize_with = "deser_flags_u32")]
    pub struct_flags: u32,
    #[serde(rename = "super")]
    pub super_struct: Option<String>,
    #[serde(default)]
    pub has_static_struct: bool,
    #[serde(default)]
    pub props: Vec<PropertyInfo>,
}

#[derive(Deserialize, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub cpp_name: String,
    pub package: String,
    pub underlying_type: String,
    pub cpp_form: u32,
    pub pairs: Vec<(String, i64)>,
}

#[derive(Deserialize, Clone)]
pub struct PropertyInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub prop_flags: u64,
    #[serde(default = "default_array_dim")]
    pub array_dim: u32,
    pub enum_name: Option<String>,
    pub enum_cpp_name: Option<String>,
    pub enum_cpp_form: Option<u32>,
    pub enum_underlying_type: Option<String>,
    pub class_name: Option<String>,
    pub meta_class_name: Option<String>,
    pub struct_name: Option<String>,
    pub interface_name: Option<String>,
    pub func_info: Option<serde_json::Value>,
    pub inner_prop: Option<Box<PropertyInfo>>,
    pub key_prop: Option<Box<PropertyInfo>>,
    pub value_prop: Option<Box<PropertyInfo>>,
    pub element_prop: Option<Box<PropertyInfo>>,
    pub getter: Option<String>,
    pub setter: Option<String>,
    pub default: Option<String>,
}

fn default_array_dim() -> u32 {
    1
}

#[derive(Deserialize, Clone)]
pub struct FunctionInfo {
    pub name: String,
    #[serde(deserialize_with = "deser_flags_u32")]
    pub func_flags: u32,
    #[serde(default)]
    pub is_static: bool,
    #[serde(default)]
    pub params: Vec<ParamInfo>,
    #[serde(skip)]
    pub ue_name: String,
    #[serde(skip)]
    pub interface: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct ParamInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub prop_flags: u64,
    pub enum_name: Option<String>,
    pub enum_cpp_name: Option<String>,
    pub enum_cpp_form: Option<u32>,
    pub enum_underlying_type: Option<String>,
    pub class_name: Option<String>,
    pub meta_class_name: Option<String>,
    pub struct_name: Option<String>,
    pub interface_name: Option<String>,
    pub func_info: Option<serde_json::Value>,
    pub inner_prop: Option<Box<PropertyInfo>>,
    pub key_prop: Option<Box<PropertyInfo>>,
    pub value_prop: Option<Box<PropertyInfo>>,
    pub element_prop: Option<Box<PropertyInfo>>,
    pub default: Option<String>,
}

pub use rusteal_ue_flags::{
    CPF_BLUEPRINT_ASSIGNABLE, CPF_BLUEPRINT_READ_ONLY, CPF_BLUEPRINT_VISIBLE, CPF_CONST_PARM,
    CPF_EDIT, CPF_NATIVE_ACCESS_SPECIFIER_PRIVATE as CPF_NATIVE_ACCESS_PRIVATE,
    CPF_NATIVE_ACCESS_SPECIFIER_PROTECTED as CPF_NATIVE_ACCESS_PROTECTED, CPF_OUT_PARM,
    CPF_REFERENCE_PARM, CPF_RETURN_PARM, FUNC_BLUEPRINT_EVENT, FUNC_EDITOR_ONLY, FUNC_NATIVE,
    FUNC_PROTECTED, FUNC_STATIC,
};

fn default_true() -> bool {
    true
}

fn deser_flags_u32<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = i64::deserialize(d)?;
    Ok(v as u32)
}
