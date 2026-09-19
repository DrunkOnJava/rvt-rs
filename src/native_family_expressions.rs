//! Bounded saved-expression trees. Operators are mapped individually from
//! their saved representation, not inferred from ordinal ordering.
use crate::{native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum Expression {
    Number {
        value: f64,
        spec_type_id: String,
        source_object: usize,
    },
    Parameter {
        parameter_id: i64,
        source_object: usize,
    },
    Function {
        function: i64,
        arguments: Vec<Expression>,
        source_object: usize,
    },
    Binary {
        operator: i64,
        left: Box<Expression>,
        right: Box<Expression>,
        source_object: usize,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum Scalar {
    Number(f64),
    Boolean(bool),
}
/// Resolve a serialized expression pointer owned by the supplied graph object.
pub fn read(graph: &ObjectGraph, owner: usize, pointer: &Value) -> Result<Expression> {
    let mut active = BTreeSet::new();
    let mut count = 0usize;
    fn node(
        graph: &ObjectGraph,
        index: usize,
        active: &mut BTreeSet<usize>,
        count: &mut usize,
        depth: usize,
    ) -> Result<Expression> {
        ensure!(
            depth <= 64 && *count < 256,
            "expression traversal budget exceeded"
        );
        *count += 1;
        ensure!(active.insert(index), "cyclic saved expression graph");
        let object = graph
            .objects
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("expression object outside graph"))?;
        let fields = &object.fields;
        let result = match object.class_name.as_str() {
            "NumberConstantExpression" => {
                let value = fields["m_value"]
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("number expression value absent"))?;
                ensure!(value.is_finite(), "nonfinite expression constant");
                Expression::Number {
                    value,
                    spec_type_id: fields["m_specTypeId"]["m_typeId"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("number expression spec absent"))?
                        .into(),
                    source_object: index,
                }
            }
            "ParameterExpression" => Expression::Parameter {
                parameter_id: identifier(&fields["m_paramId"])?,
                source_object: index,
            },
            "BinaryOperatorExpression" => Expression::Binary {
                operator: fields["m_binaryOperator"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("binary operator absent"))?,
                left: Box::new(node(
                    graph,
                    target(graph, index, &fields["m_pLeftSubexpression"])?,
                    active,
                    count,
                    depth + 1,
                )?),
                right: Box::new(node(
                    graph,
                    target(graph, index, &fields["m_pRightSubexpression"])?,
                    active,
                    count,
                    depth + 1,
                )?),
                source_object: index,
            },
            "FunctionExpression" => {
                let function = fields["m_function"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("function identifier absent"))?;
                let pointers = fields["m_subexpressions"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("function arguments absent"))?;
                let arguments = pointers
                    .iter()
                    .map(|p| node(graph, target(graph, index, p)?, active, count, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                Expression::Function {
                    function,
                    arguments,
                    source_object: index,
                }
            }
            class => anyhow::bail!("unqualified saved expression class {class}"),
        };
        active.remove(&index);
        Ok(result)
    }
    node(
        graph,
        target(graph, owner, pointer)?,
        &mut active,
        &mut count,
        0,
    )
}
fn target(graph: &ObjectGraph, owner: usize, pointer: &Value) -> Result<usize> {
    ensure!(
        pointer["pointer_token"].as_u64().is_some_and(|t| t != 0),
        "null or missing expression pointer"
    );
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("expression pointer offset absent"))?;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == owner && e.pointer_offset as u64 == offset)
        .collect();
    ensure!(
        edges.len() == 1,
        "expression pointer has no unique owning edge"
    );
    Ok(edges[0].target_object_index)
}
/// Evaluate the qualified arithmetic subset using caller-resolved saved
/// parameter identities. Parameter names are not part of this interface.
pub fn evaluate(
    expression: &Expression,
    resolve: &mut impl FnMut(i64) -> Result<Scalar>,
) -> Result<Scalar> {
    match expression {
        Expression::Number { value, .. } => Ok(Scalar::Number(*value)),
        Expression::Parameter { parameter_id, .. } => resolve(*parameter_id),
        Expression::Function {
            function,
            arguments,
            ..
        } => {
            // Parametric-family controlled fixture: opcode10 stores if(condition,true,false).
            ensure!(
                *function == 10 && arguments.len() == 3,
                "unqualified saved function or arity {function}"
            );
            let Scalar::Boolean(condition) = evaluate(&arguments[0], resolve)? else {
                anyhow::bail!("conditional requires boolean condition")
            };
            evaluate(&arguments[if condition { 1 } else { 2 }], resolve)
        }
        Expression::Binary {
            operator,
            left,
            right,
            ..
        } => {
            // Operator3 is witnessed by LAB_FORMULA_RESULT=LAB_FORMULA_INPUT*2
            // in baseline and independently changed formula_input saved states.
            ensure!(
                matches!(*operator, 1 | 3),
                "unqualified binary expression operator {operator}"
            );
            let Scalar::Number(left) = evaluate(left, resolve)? else {
                anyhow::bail!("numeric operator receives nonnumeric left operand")
            };
            let Scalar::Number(right) = evaluate(right, resolve)? else {
                anyhow::bail!("numeric operator receives nonnumeric right operand")
            };
            // Opcode1: CL Effective Depth = CL Depth + CL Instance Offset.
            let value = if *operator == 1 {
                left + right
            } else {
                left * right
            };
            ensure!(value.is_finite(), "nonfinite expression result");
            Ok(Scalar::Number(value))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addition_combines_numeric_dependencies_without_accepting_booleans() {
        let expression = Expression::Binary {
            operator: 1,
            left: Box::new(Expression::Parameter {
                parameter_id: 1,
                source_object: 1,
            }),
            right: Box::new(Expression::Parameter {
                parameter_id: 2,
                source_object: 2,
            }),
            source_object: 0,
        };
        assert_eq!(
            evaluate(&expression, &mut |id| Ok(Scalar::Number(if id == 1 {
                3.5
            } else {
                -0.75
            })))
            .unwrap(),
            Scalar::Number(2.75)
        );
        assert!(evaluate(&expression, &mut |_| Ok(Scalar::Boolean(true))).is_err());
    }
    #[test]
    fn multiplication_uses_resolved_ids_and_rejects_unknown_or_boolean_operands() {
        let mut expression = Expression::Binary {
            operator: 3,
            left: Box::new(Expression::Parameter {
                parameter_id: 47,
                source_object: 1,
            }),
            right: Box::new(Expression::Number {
                value: 2.0,
                spec_type_id: "number".into(),
                source_object: 2,
            }),
            source_object: 0,
        };
        assert_eq!(
            evaluate(&expression, &mut |id| {
                assert_eq!(id, 47);
                Ok(Scalar::Number(11.75))
            })
            .unwrap(),
            Scalar::Number(23.5)
        );
        assert!(evaluate(&expression, &mut |_| Ok(Scalar::Boolean(true))).is_err());
        if let Expression::Binary { operator, .. } = &mut expression {
            *operator = 99;
        }
        assert!(evaluate(&expression, &mut |_| Ok(Scalar::Number(1.0))).is_err());
    }
    #[test]
    fn conditional_selects_only_the_qualified_boolean_branch() {
        let expression = Expression::Function {
            function: 10,
            arguments: vec![
                Expression::Parameter {
                    parameter_id: 1,
                    source_object: 1,
                },
                Expression::Parameter {
                    parameter_id: 2,
                    source_object: 2,
                },
                Expression::Parameter {
                    parameter_id: 3,
                    source_object: 3,
                },
            ],
            source_object: 0,
        };
        for condition in [false, true] {
            let result = evaluate(&expression, &mut |id| match id {
                1 => Ok(Scalar::Boolean(condition)),
                2 if condition => Ok(Scalar::Number(12.0)),
                3 if !condition => Ok(Scalar::Number(6.0)),
                _ => anyhow::bail!("unselected branch must not be resolved"),
            })
            .unwrap();
            assert_eq!(result, Scalar::Number(if condition { 12.0 } else { 6.0 }));
        }
        assert!(evaluate(&expression, &mut |_| Ok(Scalar::Number(1.0))).is_err());
    }
    #[test]
    fn aliased_graph_cycle_is_not_recursive_evaluation() {
        let graph:ObjectGraph=serde_json::from_value(serde_json::json!({"consumed_bytes":8,"objects":[{"class_tag":12,"class_name":"BinaryOperatorExpression","token":1,"start":2,"fields_end":8,"fields":{"m_binaryOperator":3,"m_pLeftSubexpression":{"offset":4,"pointer_token":1},"m_pRightSubexpression":{"offset":6,"pointer_token":1}}}],"edges":[{"source_object_index":0,"pointer_offset":4,"pointer_token":1,"target_object_index":0,"target_class_tag":12},{"source_object_index":0,"pointer_offset":6,"pointer_token":1,"target_object_index":0,"target_class_tag":12}]})).unwrap();
        assert!(
            read(
                &graph,
                0,
                &serde_json::json!({"offset":4,"pointer_token":1})
            )
            .unwrap_err()
            .to_string()
            .contains("cyclic")
        );
    }
}
