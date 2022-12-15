
from ....rust import make_enum, make_struct, make_tuple, get_class, SelfResolvingTypeVar, Dict
import typing
            
FindAllActiveTriggerIds = make_tuple("FindAllActiveTriggerIds")
FindTriggerById = make_struct("FindTriggerById", [("id", "iroha_data_model.expression.EvaluatesTo")])

FindTriggerKeyValueByIdAndKey = make_struct("FindTriggerKeyValueByIdAndKey", [("id", "iroha_data_model.expression.EvaluatesTo"), ("key", "iroha_data_model.expression.EvaluatesTo")])

FindTriggersByDomainId = make_struct("FindTriggersByDomainId", [("domain_id", "iroha_data_model.expression.EvaluatesTo")])

SelfResolvingTypeVar.resolve_all()
