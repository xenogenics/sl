use std::ffi::c_void;

use libffi::low;

use crate::{
    error::Error,
    ir::{ExternalDefinition, ExternalType},
    opcodes::{Arity, Immediate},
    stack,
};

//
// External value.
//

#[derive(Debug)]
pub enum Value {
    Integer(i64),
    String(Box<[u8]>),
    Void,
}

impl Value {
    fn ptr(&mut self) -> *mut c_void {
        match self {
            Value::Integer(v) => v as *mut _ as *mut c_void,
            Value::String(v) => v as *mut _ as *mut c_void,
            Value::Void => panic!("Cannot create a pointer to void"),
        }
    }
}

impl TryFrom<(ExternalType, &stack::Value)> for Value {
    type Error = Error;

    fn try_from(value: (ExternalType, &stack::Value)) -> Result<Self, Self::Error> {
        match value.0 {
            ExternalType::Integer => Ok(Self::Integer(value.1.as_immediate().as_number())),
            ExternalType::String => {
                let val = value.1.as_nul_terminated_string();
                Ok(Self::String(val))
            }
            ExternalType::Void => Ok(Self::Void),
        }
    }
}

//
// Stub.
//

pub struct Stub {
    args: Vec<ExternalType>,
    #[allow(dead_code)]
    atyp: Vec<*mut low::ffi_type>,
    rtyp: ExternalType,
    cif: low::ffi_cif,
    ptr: *mut c_void,
}

impl Stub {
    pub fn arity(&self) -> Arity {
        Arity::Some(self.args.len() as u16)
    }

    pub fn call(&mut self, vals: &[stack::Value]) -> Result<stack::Value, Error> {
        //
        // Convert the argument to FFI values.
        //
        let mut values: Vec<_> = self
            .args
            .iter()
            .copied()
            .zip(vals.iter().rev())
            .map(Value::try_from)
            .collect::<Result<_, _>>()?;
        //
        // Convert FFI values to raw pointers.
        //
        let mut pointers: Vec<_> = values.iter_mut().map(|v| v.ptr()).collect();
        //
        // Call the CIF.
        //
        let result = unsafe {
            match self.rtyp {
                ExternalType::Integer => {
                    let res = low::call::<i64>(
                        &mut self.cif,
                        low::CodePtr(self.ptr),
                        pointers.as_mut_ptr(),
                    );
                    stack::Value::Immediate(Immediate::Number(res))
                }
                _ => todo!(),
            }
        };
        //
        // Done.
        //
        Ok(result)
    }

    #[allow(static_mut_refs)]
    fn as_ffi_type(v: ExternalType) -> &'static mut low::ffi_type {
        use libffi::low::types;
        unsafe {
            match v {
                ExternalType::Integer => &mut types::sint64,
                ExternalType::String => &mut types::pointer,
                ExternalType::Void => &mut types::void,
            }
        }
    }
}

impl TryFrom<ExternalDefinition> for Stub {
    type Error = Error;

    fn try_from(value: ExternalDefinition) -> Result<Self, Self::Error> {
        //
        // Open self.
        //
        let module = dlopen2::raw::Library::open_self().map_err(|_| Error::OpenSelfImage)?;
        //
        // Get the function symbol.
        //
        let ptr: *mut c_void = unsafe { module.symbol(value.symbol().as_ref()) }
            .map_err(|_| Error::UnresolvedSymbol(value.symbol().clone()))?;
        //
        // Build the arguments types.
        //
        let mut atyp: Vec<_> = value
            .arguments()
            .types()
            .map(Self::as_ffi_type)
            .map(|v| v as *mut _)
            .collect();
        //
        // Build the CIF.
        //
        let mut cif: low::ffi_cif = Default::default();
        //
        // Prepare the CIF.
        //
        unsafe {
            low::prep_cif(
                &mut cif,
                low::ffi_abi_FFI_DEFAULT_ABI,
                atyp.len(),
                Self::as_ffi_type(value.return_type()),
                atyp.as_mut_ptr(),
            )
            .unwrap();
        }
        //
        // Done.
        //
        Ok(Self {
            args: value.arguments().types().collect(),
            atyp,
            rtyp: value.return_type(),
            cif,
            ptr,
        })
    }
}
