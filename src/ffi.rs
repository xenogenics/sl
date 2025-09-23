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

#[derive(Debug, Default)]
pub enum Value {
    Bytes(*mut u8),
    Integer(i64),
    String(*mut i8),
    #[default]
    Void,
}

impl Value {
    fn ptr(&mut self) -> *mut c_void {
        match self {
            Value::Bytes(v) => v as *mut _ as *mut c_void,
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
            ExternalType::Bytes => {
                let bytes = value.1.as_mut_ptr();
                Ok(Self::Bytes(bytes))
            }
            ExternalType::Integer => Ok(Self::Integer(value.1.as_immediate().as_number())),
            ExternalType::String => {
                let cstr = value.1.as_raw_cstr();
                Ok(Self::String(cstr))
            }
            ExternalType::Void => Ok(Self::Void),
        }
    }
}

//
// Stub.
//

#[derive(Debug)]
pub struct Stub {
    args: Vec<ExternalType>,
    #[allow(dead_code)]
    atyp: Vec<*mut low::ffi_type>,
    rtyp: ExternalType,
    cif: low::ffi_cif,
    func: *mut c_void,
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
        // Collect the argument pointers.
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
                        low::CodePtr(self.func),
                        pointers.as_mut_ptr(),
                    );
                    stack::Value::Immediate(Immediate::Number(res))
                }
                ExternalType::Void => {
                    low::call::<c_void>(
                        &mut self.cif,
                        low::CodePtr(self.func),
                        pointers.as_mut_ptr(),
                    );
                    stack::Value::Immediate(Immediate::Nil)
                }
                _ => unimplemented!(),
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
                ExternalType::Bytes => &mut types::pointer,
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
        let func: *mut c_void = unsafe { module.symbol(value.symbol().as_ref()) }
            .map_err(|_| Error::UnresolvedSymbol(value.symbol().clone()))?;
        //
        // Build the argument list and get the return type.
        //
        let args: Vec<_> = value.arguments().types().collect();
        let rtyp = value.return_type();
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
            args,
            atyp,
            rtyp,
            cif,
            func,
        })
    }
}
