extern crate console_error_panic_hook;
use crate::utils::log;
use clarinet_files::{FileAccessor, FileLocation, PerformVFSAction};
use js_sys::{Function as JsFunction, Promise};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Serialize, Deserialize)]
struct VFSRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
struct VFSWriteRequest<'a> {
    pub path: String,
    pub content: &'a [u8],
}

pub struct VscodeFilesystemAccessor {
    client_request_tx: JsFunction,
}

impl VscodeFilesystemAccessor {
    pub fn new(client_request_tx: JsFunction) -> VscodeFilesystemAccessor {
        VscodeFilesystemAccessor { client_request_tx }
    }

    fn get_request_promise<T: Serialize>(
        &self,
        action: String,
        data: &T,
    ) -> Result<JsValue, JsValue> {
        self.client_request_tx.call2(
            &JsValue::null(),
            &JsValue::from(action),
            &encode_to_js(data)?,
        )
    }
}

impl FileAccessor for VscodeFilesystemAccessor {
    fn file_exists(&self, location: FileLocation) -> PerformVFSAction<bool> {
        let file_exists_promise = self.get_request_promise(
            "vfs/exists".into(),
            &VFSRequest {
                path: location.to_string(),
            },
        );

        Box::pin(async move {
            match file_exists_promise {
                Ok(promise) => match JsFuture::from(Promise::resolve(&promise)).await {
                    Ok(res) => Ok(res.is_truthy()),
                    Err(_) => Err("error".into()),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }

    fn read_manifest_content(
        &self,
        manifest_location: FileLocation,
    ) -> PerformVFSAction<(FileLocation, String)> {
        log!("reading manifest");
        let read_file_promise = self.get_request_promise(
            "vfs/readFile".into(),
            &VFSRequest {
                path: manifest_location.to_string(),
            },
        );

        Box::pin(async move {
            match read_file_promise {
                Ok(req) => match JsFuture::from(Promise::resolve(&req)).await {
                    Ok(manifest) => Ok((
                        manifest_location,
                        decode_from_js(manifest)
                            .map_err(|err| format!("decode_from_js error: {:?}", err))?,
                    )),
                    Err(_) => Err("error".into()),
                },
                Err(_) => Err("error".into()),
            }
        })
    }

    fn read_contract_content(
        &self,
        manifest_location: FileLocation,
        relative_path: String,
    ) -> PerformVFSAction<(FileLocation, String)> {
        log!("reading contract");
        let req = (|| -> Result<(FileLocation, JsValue), String> {
            let mut contract_location = manifest_location.get_parent_location()?;
            contract_location.append_path(&relative_path)?;

            let req = self
                .get_request_promise(
                    "vfs/readFile".into(),
                    &VFSRequest {
                        path: contract_location.to_string(),
                    },
                )
                .map_err(|_| "failed to read_file")?;

            Ok((contract_location, req))
        })();

        Box::pin(async move {
            match req {
                Ok((contract_location, req)) => {
                    match JsFuture::from(Promise::resolve(&req)).await {
                        Ok(contract) => Ok((contract_location, decode_from_js(contract).unwrap())),
                        Err(_) => Err("error".into()),
                    }
                }
                Err(_) => Err("error".into()),
            }
        })
    }

    fn write_file(
        &self,
        _manifest_location: FileLocation,
        relative_path: String,
        content: &[u8],
    ) -> PerformVFSAction<()> {
        log!("writting contract");
        let write_file_promise = self.get_request_promise(
            "vfs/writeFile".into(),
            &VFSWriteRequest {
                path: relative_path.to_string(),
                content,
            },
        );

        Box::pin(async move {
            match write_file_promise {
                Ok(promise) => match JsFuture::from(Promise::resolve(&promise)).await {
                    Ok(_) => Ok(()),
                    Err(_) => Err("error".into()),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }
}