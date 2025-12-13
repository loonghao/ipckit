//! Python bindings for SharedMemory
//!
//! This module provides Python bindings for shared memory operations.

use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::shm::SharedMemory as RustSharedMemory;

/// Python wrapper for SharedMemory
#[pyclass(name = "SharedMemory")]
pub struct PySharedMemory {
    inner: RustSharedMemory,
}

#[pymethods]
impl PySharedMemory {
    /// Create a new shared memory region
    #[staticmethod]
    fn create(name: &str, size: usize) -> PyResult<Self> {
        let inner = RustSharedMemory::create(name, size)?;
        Ok(Self { inner })
    }

    /// Open an existing shared memory region
    #[staticmethod]
    fn open(name: &str) -> PyResult<Self> {
        let inner = RustSharedMemory::open(name)?;
        Ok(Self { inner })
    }

    /// Get the shared memory name
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the shared memory size
    #[getter]
    fn size(&self) -> usize {
        self.inner.size()
    }

    /// Check if this instance is the owner
    #[getter]
    fn is_owner(&self) -> bool {
        self.inner.is_owner()
    }

    /// Write data to shared memory at offset
    fn write(&mut self, offset: usize, data: &[u8]) -> PyResult<()> {
        self.inner.write(offset, data)?;
        Ok(())
    }

    /// Read data from shared memory at offset
    fn read(&self, py: Python<'_>, offset: usize, size: usize) -> PyResult<Py<PyBytes>> {
        let data = self.inner.read(offset, size)?;
        Ok(PyBytes::new(py, &data).into())
    }

    /// Read all data from shared memory
    fn read_all(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        let data = self.inner.read(0, self.inner.size())?;
        Ok(PyBytes::new(py, &data).into())
    }
}
