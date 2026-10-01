use syn::{Ident, Path, PathArguments, PathSegment, Type};

pub trait TypeExt {
    fn is_ident<I: ?Sized>(&self, ident: &I) -> bool
    where
        Ident: PartialEq<I>;

    fn is_list_collection(&self) -> bool;

    fn is_numeric_type(&self) -> bool;

    fn get_inner_type(&self) -> Option<&Type>;
}

impl TypeExt for Type {
    fn is_ident<I: ?Sized>(&self, ident: &I) -> bool
    where
        Ident: PartialEq<I>,
    {
        let mut ty = self;
        if single_argument_path_segment(ty)
            .map(|last| <syn::Ident as PartialEq<str>>::eq(&last.ident, "Option"))
            .unwrap_or(false)
        {
            ty = self.get_inner_type().unwrap();
        }
        let Some(path) = type_path(ty) else {
            return false;
        };
        let Some(last) = path.segments.last() else {
            return false;
        };

        last.ident == *ident
    }

    fn is_list_collection(&self) -> bool {
        let Some(last) = single_argument_path_segment(self) else {
            return false;
        };

        pub(crate) const LIST_COLLECTION_TYPES: [&str; 6] = [
            "Vec",
            "LinkedList",
            "VecDeque",
            "BinaryHeap",
            "HashSet",
            "BTreeSet",
        ];

        LIST_COLLECTION_TYPES.contains(&last.ident.to_string().as_str())
    }

    fn is_numeric_type(&self) -> bool {
        let Some(ident) = type_path(self)
            .and_then(|path| path.segments.last())
            .map(|v| v.ident.to_string())
        else {
            return false;
        };

        matches!(
            ident.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
        )
    }

    fn get_inner_type(&self) -> Option<&syn::Type> {
        let path = type_path(self)?;
        let last = path.segments.last()?;

        if let PathArguments::AngleBracketed(args) = &last.arguments
            && let Some(syn::GenericArgument::Type(ty)) = args.args.first()
        {
            return Some(ty);
        }

        None
    }
}

fn type_path(ty: &Type) -> Option<&Path> {
    match ty {
        Type::Path(ty) => Some(&ty.path),
        _ => None,
    }
}

fn single_argument_path_segment(ty: &Type) -> Option<&PathSegment> {
    let last = type_path(ty)?.segments.last()?;
    match &last.arguments {
        PathArguments::AngleBracketed(args) if args.args.len() == 1 => Some(last),
        _ => None,
    }
}

#[test]
fn test_is_list_collection() {
    let ty: syn::Type = syn::parse_str("Vec<String>").unwrap();
    assert_eq!(ty.is_list_collection(), true);
    let ty: syn::Type = syn::parse_str("HashSet<u32>").unwrap();
    assert_eq!(ty.is_list_collection(), true);
    let ty: syn::Type = syn::parse_str("std::vec::Vec<String>").unwrap();
    assert_eq!(ty.is_list_collection(), true);

    let ty: syn::Type = syn::parse_str("Vec<>").unwrap();
    assert_eq!(ty.is_list_collection(), false);
    let ty: syn::Type = syn::parse_str("Vec<String, u8>").unwrap();
    assert_eq!(ty.is_list_collection(), false);
    let ty: syn::Type = syn::parse_str("(String, u8)").unwrap();
    assert_eq!(ty.is_list_collection(), false);
}

#[test]
fn test_is_ident_option() {
    let ty: syn::Type = syn::parse_str("Option<String>").unwrap();
    assert_eq!(ty.is_ident("String"), true);
    assert_eq!(ty.is_ident("Option"), false);
    let ty: syn::Type = syn::parse_str("std::option::Option<String>").unwrap();
    assert_eq!(ty.is_ident("String"), true);

    let ty: syn::Type = syn::parse_str("Option<>").unwrap();
    assert_eq!(ty.is_ident("Option"), true);
    let ty: syn::Type = syn::parse_str("Option<String, u8>").unwrap();
    assert_eq!(ty.is_ident("Option"), true);
}

#[test]
fn test_is_numeric() {
    let ty: syn::Type = syn::parse_str("i32").unwrap();
    assert_eq!(ty.is_numeric_type(), true);
    let ty: syn::Type = syn::parse_str("usize").unwrap();
    assert_eq!(ty.is_numeric_type(), true);
}

#[test]
fn test_get_inner_type() {
    let ty = syn::parse_str::<Type>("Vec<u32>").unwrap();
    let inner = ty.get_inner_type();
    assert!(inner.map(|v| v.is_ident("u32")).unwrap());
    let ty = syn::parse_str::<Type>("HashSet<String>").unwrap();
    let inner = ty.get_inner_type();
    assert!(inner.map(|v| v.is_ident("String")).unwrap());
    let ty = syn::parse_str::<Type>("std::vec::Vec<String>").unwrap();
    let inner = ty.get_inner_type();
    assert!(inner.map(|v| v.is_ident("String")).unwrap());
}
