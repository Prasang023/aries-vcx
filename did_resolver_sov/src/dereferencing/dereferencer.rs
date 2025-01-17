use std::io::Cursor;

use async_trait::async_trait;
use did_resolver::{
    did_parser::DidUrl,
    error::GenericError,
    traits::{
        dereferenceable::{
            dereferencing_options::DidDereferencingOptions,
            dereferencing_output::DidDereferencingOutput, DidDereferenceable,
        },
        resolvable::{resolution_options::DidResolutionOptions, DidResolvable},
    },
};

use crate::resolution::DidSovResolver;

use super::utils::dereference_did_document;

#[async_trait]
impl DidDereferenceable for DidSovResolver {
    type Output = Cursor<Vec<u8>>;

    async fn dereference(
        &self,
        did_url: &DidUrl,
        _options: &DidDereferencingOptions,
    ) -> Result<DidDereferencingOutput<Self::Output>, GenericError> {
        let resolution_output = self
            .resolve(&did_url.try_into()?, &DidResolutionOptions::default())
            .await?;

        dereference_did_document(&resolution_output, did_url).map_err(|err| err.into())
    }
}
