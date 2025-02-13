//! Query module provides [`Query`] Transaction related implementations.

use std::sync::Arc;

use eyre::{Result, WrapErr};
use iroha_crypto::HashOf;
use iroha_data_model::{
    block::SignedBlock,
    evaluate::ExpressionEvaluator,
    prelude::*,
    query::{
        error::{FindError, QueryExecutionFail},
        TransactionQueryOutput,
    },
    transaction::TransactionValue,
};
use iroha_telemetry::metrics;

use super::*;

pub(crate) struct BlockTransactionIter(Arc<SignedBlock>, usize);
pub(crate) struct BlockTransactionRef(Arc<SignedBlock>, usize);

impl BlockTransactionIter {
    fn new(block: Arc<SignedBlock>) -> Self {
        let n_transactions = block.payload().transactions.len();
        Self(block, n_transactions)
    }
}

impl Iterator for BlockTransactionIter {
    type Item = BlockTransactionRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 != 0 {
            self.1 -= 1;
            return Some(BlockTransactionRef(Arc::clone(&self.0), self.1));
        }
        
        None
    }
}

impl BlockTransactionRef {
    fn block_hash(&self) -> HashOf<SignedBlock> {
        self.0.hash()
    }

    fn authority(&self) -> &AccountId {
        &self.0.payload().transactions[self.1].payload().authority
    }
    fn value(&self) -> TransactionValue {
        self.0.payload().transactions[self.1].clone()
    }
}

impl ValidQuery for FindAllTransactions {
    #[metrics(+"find_all_transactions")]
    fn execute<'wsv>(
        &self,
        wsv: &'wsv WorldStateView,
    ) -> Result<Box<dyn Iterator<Item = TransactionQueryOutput> + 'wsv>, QueryExecutionFail> {
        Ok(Box::new(
            wsv.all_blocks()
                .rev()
                .flat_map(BlockTransactionIter::new)
                .map(|tx| TransactionQueryOutput {
                    block_hash: tx.block_hash(),
                    transaction: tx.value(),
                }),
        ))
    }
}

impl ValidQuery for FindTransactionsByAccountId {
    #[metrics(+"find_transactions_by_account_id")]
    fn execute<'wsv>(
        &self,
        wsv: &'wsv WorldStateView,
    ) -> Result<Box<dyn Iterator<Item = TransactionQueryOutput> + 'wsv>, QueryExecutionFail> {
        let account_id = wsv
            .evaluate(&self.account_id)
            .wrap_err("Failed to get account id")
            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

        Ok(Box::new(
            wsv.all_blocks()
                .rev()
                .flat_map(BlockTransactionIter::new)
                .filter(move |tx| *tx.authority() == account_id)
                .map(|tx| TransactionQueryOutput {
                    block_hash: tx.block_hash(),
                    transaction: tx.value(),
                }),
        ))
    }
}

fn is_transaction_related_to_account(
    wsv: &WorldStateView,
    tx: &BlockTransactionRef,
    account_id: &AccountId,
) -> Result<bool, QueryExecutionFail> {
    if *tx.authority() == *account_id {
        return Ok(true);
    }

    let binding = tx.value();
    let payload = binding.payload();

    let metadata = &payload.metadata;
    let option_value = metadata.get("involved_accounts");
    if option_value.is_some() {
        match option_value {
            Some(value) => match value {
                Value::Vec(vec_value) => {
                    for value in vec_value {
                        match value {
                            Value::Id(id) => match id {
                                IdBox::AccountId(involved_account_id) => {
                                    if involved_account_id == account_id {
                                        return Ok(true);
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            None => {}
        }
    }

    let executable: &Executable = payload.instructions();
    match executable {
        Executable::Instructions(instructions) => {
            for instruction_expr in instructions {
                match instruction_expr {
                    InstructionExpr::Transfer(transfer_expr) => {
                        let source_id_box = wsv
                            .evaluate(&transfer_expr.source_id)
                            .wrap_err("failed to evaluate source_id")
                            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

                        match source_id_box {
                            IdBox::AssetId(asset_id) => {
                                if asset_id.account_id == *account_id {
                                    return Ok(true);
                                }
                            }
                            _ => {}
                        }

                        let destination_id_box = wsv
                            .evaluate(&transfer_expr.destination_id)
                            .wrap_err("failed to evaluate destination_id")
                            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

                        match destination_id_box {
                            IdBox::AccountId(involved_account_id) => {
                                if involved_account_id == *account_id {
                                    return Ok(true);
                                }
                            }
                            _ => {}
                        }
                    }
                    InstructionExpr::Register(register_expr) => {
                        let registrable_box = wsv
                            .evaluate(&register_expr.object)
                            .wrap_err("failed to evaluate account_id")
                            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

                        match registrable_box {
                            RegistrableBox::Account(new_account) => {
                                if new_account.id == *account_id {
                                    return Ok(true);
                                }
                            }
                            _ => {}
                        }
                    }
                    InstructionExpr::Mint(mint_expr) => {
                        let value = wsv
                            .evaluate(&mint_expr.destination_id)
                            .wrap_err("failed to evaluate asset_id")
                            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

                        match value {
                            IdBox::AssetId(asset_id) => {
                                if asset_id.account_id == *account_id {
                                    return Ok(true);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

impl ValidQuery for FindTransactionsByAccountIdInvolved {
    #[metrics(+"find_transactions_by_account_id_involved")]
    fn execute<'wsv>(
        &self,
        wsv: &'wsv WorldStateView,
    ) -> Result<Box<dyn Iterator<Item = TransactionQueryOutput> + 'wsv>, QueryExecutionFail> {
        let account_id = wsv
            .evaluate(&self.account_id)
            .wrap_err("Failed to get account id")
            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;

        Ok(Box::new(
            wsv.all_blocks()
                .rev()
                .flat_map(BlockTransactionIter::new)
                .filter_map(move |tx| {
                    match is_transaction_related_to_account(wsv, &tx, &account_id) {
                        Ok(true) => Some(TransactionQueryOutput {
                            block_hash: tx.block_hash(),
                            transaction: tx.value(),
                        }),
                        Ok(false) => None,
                        Err(e) => {
                            iroha_logger::error!("Error evaluating transaction: {}", e);
                            panic!("Error evaluating transaction: {}", e);
                        }
                    }
                }),
        ))
    }
}

impl ValidQuery for FindTransactionByHash {
    #[metrics(+"find_transaction_by_hash")]
    fn execute(&self, wsv: &WorldStateView) -> Result<TransactionQueryOutput, QueryExecutionFail> {
        let tx_hash = wsv
            .evaluate(&self.hash)
            .wrap_err("Failed to get hash")
            .map_err(|e| QueryExecutionFail::Evaluate(e.to_string()))?;
        iroha_logger::trace!(%tx_hash);
        if !wsv.has_transaction(tx_hash) {
            return Err(FindError::Transaction(tx_hash).into());
        };
        let block = wsv
            .block_with_tx(&tx_hash)
            .ok_or_else(|| FindError::Transaction(tx_hash))?;

        let block_hash = block.hash();

        block
            .payload()
            .transactions
            .iter()
            .find(|transaction| transaction.value.hash() == tx_hash)
            .cloned()
            .map(|transaction| TransactionQueryOutput {
                block_hash,
                transaction,
            })
            .ok_or_else(|| FindError::Transaction(tx_hash).into())
    }
}
