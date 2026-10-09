use rusteal_core::{FName, OwnedStruct, UObjectRef, UStructRef, UeStruct};

use crate::engine::{DataTable, FDataTableRowHandle, FDataTableRowHandleExt};

pub fn find_data_table_row<T: UeStruct>(
    table: UObjectRef<DataTable>,
    row_name: FName,
) -> Option<UStructRef<T>> {
    let row = rusteal_core::world::find_data_table_row_raw(
        table.raw(),
        row_name.handle(),
        T::static_struct(),
    )?;

    Some(unsafe { UStructRef::from_raw(row) })
}

pub trait DataTableRowHandleRowExt {
    fn get_row<T: UeStruct>(&self) -> Option<UStructRef<T>>;
}

impl DataTableRowHandleRowExt for UStructRef<FDataTableRowHandle> {
    fn get_row<T: UeStruct>(&self) -> Option<UStructRef<T>> {
        find_data_table_row(self.get_data_table(), FName(self.get_row_name()))
    }
}

impl DataTableRowHandleRowExt for OwnedStruct<FDataTableRowHandle> {
    fn get_row<T: UeStruct>(&self) -> Option<UStructRef<T>> {
        self.as_ref().get_row()
    }
}
