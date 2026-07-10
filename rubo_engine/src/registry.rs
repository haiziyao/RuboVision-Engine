use crate::{
    Device, DeviceRegister, Function, FunctionRegister, Sink, SinkRegister, SourceFactory,
    SourceRegister,
};

pub struct SourceInventoryRegistration {
    pub kind: &'static str,
    pub register: fn(&mut SourceRegister, &'static str),
}

pub struct DeviceInventoryRegistration {
    pub kind: &'static str,
    pub register: fn(&mut DeviceRegister, &'static str),
}

pub struct FunctionInventoryRegistration {
    pub id: &'static str,
    pub register: fn(&mut FunctionRegister, &'static str),
}

pub struct SinkInventoryRegistration {
    pub id: &'static str,
    pub register: fn(&mut SinkRegister, &'static str),
}

inventory::collect!(SourceInventoryRegistration);
inventory::collect!(DeviceInventoryRegistration);
inventory::collect!(FunctionInventoryRegistration);
inventory::collect!(SinkInventoryRegistration);

pub fn register_inventory(
    sources: &mut SourceRegister,
    devices: &mut DeviceRegister,
    functions: &mut FunctionRegister,
    sinks: &mut SinkRegister,
) {
    for registration in inventory::iter::<SourceInventoryRegistration> {
        (registration.register)(sources, registration.kind);
    }
    for registration in inventory::iter::<DeviceInventoryRegistration> {
        (registration.register)(devices, registration.kind);
    }
    for registration in inventory::iter::<FunctionInventoryRegistration> {
        (registration.register)(functions, registration.id);
    }
    for registration in inventory::iter::<SinkInventoryRegistration> {
        (registration.register)(sinks, registration.id);
    }
}

pub fn register_source_factory<T>(register: &mut SourceRegister, kind: &'static str)
where
    T: SourceFactory + Default,
{
    register.register(kind, T::default());
}

pub fn register_device_type<T>(register: &mut DeviceRegister, kind: &'static str)
where
    T: Device,
{
    register.register_device::<T>(kind);
}

pub fn register_function_type<T>(register: &mut FunctionRegister, id: &'static str)
where
    T: Function + Default,
{
    register.register(id, T::default());
}

pub fn register_sink_type<T>(register: &mut SinkRegister, id: &'static str)
where
    T: Sink + Default,
{
    register.register(id, T::default());
}
