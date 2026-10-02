// Data table rows as the structs they are: a `#[ustruct]` declared in Rust, or
// an engine struct.

use rusteal_core::{FName, OwnedStruct, UObjectRef, UStructRef, UeStruct};

use crate::engine::{DataTable, FDataTableRowHandle, FDataTableRowHandleExt};

/// The row `row_name` of `table` as a `T`, as C++'s `UDataTable::FindRow<T>`:
/// `None` when the table is null, has no such row or its rows are another
/// struct. The row is the table's memory: it lives as long as the table.
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

/// The row an `FDataTableRowHandle` names, as C++'s
/// `FDataTableRowHandle::GetRow<T>`.
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
