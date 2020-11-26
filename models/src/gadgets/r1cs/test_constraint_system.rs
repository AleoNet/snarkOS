// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    curves::Field,
    gadgets::r1cs::{ConstraintSystem, Index, LinearCombination, Variable},
};
use snarkos_errors::gadgets::SynthesisError;

use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Debug)]
enum NamedObject {
    Constraint(usize),
    Var(Variable),
    Namespace,
}

type TestConstraint<T> = (LinearCombination<T>, LinearCombination<T>, LinearCombination<T>, String);

/// Constraint system for testing purposes.
pub struct TestConstraintSystem<F: Field> {
    named_objects: BTreeMap<String, NamedObject>,
    current_namespace: Vec<String>,
    pub constraints: Vec<TestConstraint<F>>,
    inputs: Vec<(F, String)>,
    aux: Vec<(F, String)>,
}

impl<F: Field> TestConstraintSystem<F> {
    fn eval_lc(terms: &[(Variable, F)], inputs: &[(F, String)], aux: &[(F, String)]) -> F {
        let mut acc = F::zero();

        for &(var, ref coeff) in terms {
            let mut tmp = match var.get_unchecked() {
                Index::Input(index) => inputs[index].0,
                Index::Aux(index) => aux[index].0,
            };

            tmp.mul_assign(&coeff);
            acc.add_assign(&tmp);
        }

        acc
    }
}

impl<F: Field> Default for TestConstraintSystem<F> {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert("ONE".into(), NamedObject::Var(TestConstraintSystem::<F>::one()));

        TestConstraintSystem {
            named_objects: map,
            current_namespace: vec![],
            constraints: vec![],
            inputs: vec![(F::one(), "ONE".into())],
            aux: vec![],
        }
    }
}

impl<F: Field> TestConstraintSystem<F> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn print_named_objects(&self) {
        for &(_, _, _, ref name) in &self.constraints {
            println!("{}", name);
        }
    }

    pub fn which_is_unsatisfied(&self) -> Option<&str> {
        for &(ref a, ref b, ref c, ref path) in &self.constraints {
            let mut a = Self::eval_lc(a.as_ref(), &self.inputs, &self.aux);
            let b = Self::eval_lc(b.as_ref(), &self.inputs, &self.aux);
            let c = Self::eval_lc(c.as_ref(), &self.inputs, &self.aux);

            a.mul_assign(&b);

            if a != c {
                return Some(&*path);
            }
        }

        None
    }

    pub fn is_satisfied(&self) -> bool {
        self.which_is_unsatisfied().is_none()
    }

    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    pub fn set(&mut self, path: &str, to: F) {
        match self.named_objects.get(path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs[index].0 = to,
                Index::Aux(index) => self.aux[index].0 = to,
            },
            Some(e) => panic!(
                "tried to set path `{}` to value, but `{:?}` already exists there.",
                path, e
            ),
            _ => panic!("no variable exists at path: {}", path),
        }
    }

    pub fn get(&mut self, path: &str) -> F {
        match self.named_objects.get(path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs[index].0,
                Index::Aux(index) => self.aux[index].0,
            },
            Some(e) => panic!(
                "tried to get value of path `{}`, but `{:?}` exists there (not a variable)",
                path, e
            ),
            _ => panic!("no variable exists at path: {}", path),
        }
    }

    fn set_named_obj(&mut self, path: String, to: NamedObject) {
        match self.named_objects.entry(path) {
            Entry::Vacant(e) => {
                e.insert(to);
            }
            Entry::Occupied(e) => {
                let (path, _) = e.remove_entry();
                panic!("tried to create object at existing path: {}", path);
            }
        }
    }
}

fn compute_path(ns: &[String], this: &str) -> String {
    if this.contains('/') {
        panic!("'/' is not allowed in names");
    }

    // preallocate the target path size, including the separators
    let len = ns.iter().map(|s| s.len()).sum::<usize>() + ns.len() + this.len();
    let mut name = String::with_capacity(len);

    for (i, ns) in ns.iter().map(|s| s.as_str()).chain(Some(this)).enumerate() {
        if i != 0 {
            name.push('/');
        }
        name.push_str(ns);
    }

    name
}

impl<F: Field> ConstraintSystem<F> for TestConstraintSystem<F> {
    type Root = Self;

    fn alloc<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        let index = self.aux.len();
        let path = compute_path(&self.current_namespace, &annotation().into());
        self.aux.push((f()?, path.clone()));
        let var = Variable::new_unchecked(Index::Aux(index));
        self.set_named_obj(path, NamedObject::Var(var));

        Ok(var)
    }

    fn alloc_input<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        let index = self.inputs.len();
        let path = compute_path(&self.current_namespace, &annotation().into());
        self.inputs.push((f()?, path.clone()));
        let var = Variable::new_unchecked(Index::Input(index));
        self.set_named_obj(path, NamedObject::Var(var));

        Ok(var)
    }

    fn enforce<A, AR, LA, LB, LC>(&mut self, annotation: A, a: LA, b: LB, c: LC)
    where
        A: FnOnce() -> AR,
        AR: Into<String>,
        LA: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LB: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LC: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
    {
        let path = compute_path(&self.current_namespace, &annotation().into());
        let index = self.constraints.len();
        self.set_named_obj(path.clone(), NamedObject::Constraint(index));

        let mut a = a(LinearCombination::zero());
        let mut b = b(LinearCombination::zero());
        let mut c = c(LinearCombination::zero());
        a.0.shrink_to_fit();
        b.0.shrink_to_fit();
        c.0.shrink_to_fit();

        self.constraints.push((a, b, c, path));
    }

    fn push_namespace<NR: Into<String>, N: FnOnce() -> NR>(&mut self, name_fn: N) {
        let name = name_fn().into();
        let path = compute_path(&self.current_namespace, &name);
        self.set_named_obj(path, NamedObject::Namespace);
        self.current_namespace.push(name);
    }

    fn pop_namespace(&mut self) {
        assert!(self.current_namespace.pop().is_some());
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.constraints.len()
    }
}
