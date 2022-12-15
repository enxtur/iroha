
from ...rust import make_enum, make_struct, make_tuple, get_class, SelfResolvingTypeVar, Dict
import typing
            
PaginatedQueryResult = make_struct("PaginatedQueryResult", [("result", "iroha_data_model.query.QueryResult"), ("filter", "iroha_data_model.predicate.GenericPredicateBox"), ("pagination", "iroha_data_model.pagination.Pagination"), ("sorting", "iroha_data_model.sorting.Sorting"), ("total", int)])

Payload = make_struct("Payload", [("timestamp_ms", "Compact"), ("query", "iroha_data_model.query.QueryBox"), ("account_id", "iroha_data_model.account.Id"), ("filter", "iroha_data_model.predicate.GenericPredicateBox")])

QueryBox = make_enum("QueryBox", [("FindAllAccounts", get_class("iroha_data_model.query.account.FindAllAccounts")), ("FindAccountById", get_class("iroha_data_model.query.account.FindAccountById")), ("FindAccountKeyValueByIdAndKey", get_class("iroha_data_model.query.account.FindAccountKeyValueByIdAndKey")), ("FindAccountsByName", get_class("iroha_data_model.query.account.FindAccountsByName")), ("FindAccountsByDomainId", get_class("iroha_data_model.query.account.FindAccountsByDomainId")), ("FindAccountsWithAsset", get_class("iroha_data_model.query.account.FindAccountsWithAsset")), ("FindAllAssets", get_class("iroha_data_model.query.asset.FindAllAssets")), ("FindAllAssetsDefinitions", get_class("iroha_data_model.query.asset.FindAllAssetsDefinitions")), ("FindAssetById", get_class("iroha_data_model.query.asset.FindAssetById")), ("FindAssetDefinitionById", get_class("iroha_data_model.query.asset.FindAssetDefinitionById")), ("FindAssetsByName", get_class("iroha_data_model.query.asset.FindAssetsByName")), ("FindAssetsByAccountId", get_class("iroha_data_model.query.asset.FindAssetsByAccountId")), ("FindAssetsByAssetDefinitionId", get_class("iroha_data_model.query.asset.FindAssetsByAssetDefinitionId")), ("FindAssetsByDomainId", get_class("iroha_data_model.query.asset.FindAssetsByDomainId")), ("FindAssetsByDomainIdAndAssetDefinitionId", get_class("iroha_data_model.query.asset.FindAssetsByDomainIdAndAssetDefinitionId")), ("FindAssetQuantityById", get_class("iroha_data_model.query.asset.FindAssetQuantityById")), ("FindTotalAssetQuantityByAssetDefinitionId", get_class("iroha_data_model.query.asset.FindTotalAssetQuantityByAssetDefinitionId")), ("FindAssetKeyValueByIdAndKey", get_class("iroha_data_model.query.asset.FindAssetKeyValueByIdAndKey")), ("FindAssetDefinitionKeyValueByIdAndKey", get_class("iroha_data_model.query.asset.FindAssetDefinitionKeyValueByIdAndKey")), ("FindAllDomains", get_class("iroha_data_model.query.domain.FindAllDomains")), ("FindDomainById", get_class("iroha_data_model.query.domain.FindDomainById")), ("FindDomainKeyValueByIdAndKey", get_class("iroha_data_model.query.domain.FindDomainKeyValueByIdAndKey")), ("FindAllPeers", get_class("iroha_data_model.query.peer.FindAllPeers")), ("FindAllBlocks", get_class("iroha_data_model.query.block.FindAllBlocks")), ("FindAllBlockHeaders", get_class("iroha_data_model.query.block.FindAllBlockHeaders")), ("FindBlockHeaderByHash", get_class("iroha_data_model.query.block.FindBlockHeaderByHash")), ("FindAllTransactions", get_class("iroha_data_model.query.transaction.FindAllTransactions")), ("FindTransactionsByAccountId", get_class("iroha_data_model.query.transaction.FindTransactionsByAccountId")), ("FindTransactionByHash", get_class("iroha_data_model.query.transaction.FindTransactionByHash")), ("FindPermissionTokensByAccountId", get_class("iroha_data_model.query.permissions.FindPermissionTokensByAccountId")), ("FindAllPermissionTokenDefinitions", get_class("iroha_data_model.query.permissions.FindAllPermissionTokenDefinitions")), ("FindAllActiveTriggerIds", get_class("iroha_data_model.query.trigger.FindAllActiveTriggerIds")), ("FindTriggerById", get_class("iroha_data_model.query.trigger.FindTriggerById")), ("FindTriggerKeyValueByIdAndKey", get_class("iroha_data_model.query.trigger.FindTriggerKeyValueByIdAndKey")), ("FindTriggersByDomainId", get_class("iroha_data_model.query.trigger.FindTriggersByDomainId")), ("FindAllRoles", get_class("iroha_data_model.query.role.FindAllRoles")), ("FindAllRoleIds", get_class("iroha_data_model.query.role.FindAllRoleIds")), ("FindRoleByRoleId", get_class("iroha_data_model.query.role.FindRoleByRoleId")), ("FindRolesByAccountId", get_class("iroha_data_model.query.role.FindRolesByAccountId"))], typing.Union[get_class("iroha_data_model.query.account.FindAllAccounts"), get_class("iroha_data_model.query.account.FindAccountById"), get_class("iroha_data_model.query.account.FindAccountKeyValueByIdAndKey"), get_class("iroha_data_model.query.account.FindAccountsByName"), get_class("iroha_data_model.query.account.FindAccountsByDomainId"), get_class("iroha_data_model.query.account.FindAccountsWithAsset"), get_class("iroha_data_model.query.asset.FindAllAssets"), get_class("iroha_data_model.query.asset.FindAllAssetsDefinitions"), get_class("iroha_data_model.query.asset.FindAssetById"), get_class("iroha_data_model.query.asset.FindAssetDefinitionById"), get_class("iroha_data_model.query.asset.FindAssetsByName"), get_class("iroha_data_model.query.asset.FindAssetsByAccountId"), get_class("iroha_data_model.query.asset.FindAssetsByAssetDefinitionId"), get_class("iroha_data_model.query.asset.FindAssetsByDomainId"), get_class("iroha_data_model.query.asset.FindAssetsByDomainIdAndAssetDefinitionId"), get_class("iroha_data_model.query.asset.FindAssetQuantityById"), get_class("iroha_data_model.query.asset.FindTotalAssetQuantityByAssetDefinitionId"), get_class("iroha_data_model.query.asset.FindAssetKeyValueByIdAndKey"), get_class("iroha_data_model.query.asset.FindAssetDefinitionKeyValueByIdAndKey"), get_class("iroha_data_model.query.domain.FindAllDomains"), get_class("iroha_data_model.query.domain.FindDomainById"), get_class("iroha_data_model.query.domain.FindDomainKeyValueByIdAndKey"), get_class("iroha_data_model.query.peer.FindAllPeers"), get_class("iroha_data_model.query.block.FindAllBlocks"), get_class("iroha_data_model.query.block.FindAllBlockHeaders"), get_class("iroha_data_model.query.block.FindBlockHeaderByHash"), get_class("iroha_data_model.query.transaction.FindAllTransactions"), get_class("iroha_data_model.query.transaction.FindTransactionsByAccountId"), get_class("iroha_data_model.query.transaction.FindTransactionByHash"), get_class("iroha_data_model.query.permissions.FindPermissionTokensByAccountId"), get_class("iroha_data_model.query.permissions.FindAllPermissionTokenDefinitions"), get_class("iroha_data_model.query.trigger.FindAllActiveTriggerIds"), get_class("iroha_data_model.query.trigger.FindTriggerById"), get_class("iroha_data_model.query.trigger.FindTriggerKeyValueByIdAndKey"), get_class("iroha_data_model.query.trigger.FindTriggersByDomainId"), get_class("iroha_data_model.query.role.FindAllRoles"), get_class("iroha_data_model.query.role.FindAllRoleIds"), get_class("iroha_data_model.query.role.FindRoleByRoleId"), get_class("iroha_data_model.query.role.FindRolesByAccountId")])

QueryResult = make_tuple("QueryResult", ["iroha_data_model.Value"])
SignedQueryRequest = make_struct("SignedQueryRequest", [("payload", "iroha_data_model.query.Payload"), ("signature", "iroha_crypto.signature.SignatureOf")])

VersionedPaginatedQueryResult = make_enum("VersionedPaginatedQueryResult", [("V1", get_class("iroha_data_model.query.PaginatedQueryResult"))], typing.Union[get_class("iroha_data_model.query.PaginatedQueryResult")])

VersionedSignedQueryRequest = make_enum("VersionedSignedQueryRequest", [("V1", get_class("iroha_data_model.query.SignedQueryRequest"))], typing.Union[get_class("iroha_data_model.query.SignedQueryRequest")])

SelfResolvingTypeVar.resolve_all()
