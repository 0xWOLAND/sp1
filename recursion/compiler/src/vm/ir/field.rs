use crate::syn::AlgebraicVariable;
use crate::syn::Expression;
use crate::syn::FieldVariable;
use crate::syn::FromConstant;
use crate::syn::SizedVariable;
use crate::syn::Variable;
use crate::vm::AsmInstruction;
use crate::vm::Symbolic;
use crate::vm::VmBuilder;
use core::marker::PhantomData;
use p3_field::AbstractField;

use core::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Felt<F>(pub(crate) i32, pub(crate) PhantomData<F>);

impl<B: VmBuilder> Variable<B> for Felt<B::F> {
    fn uninit(builder: &mut B) -> Self {
        Felt(builder.get_mem(4), PhantomData)
    }
}

impl<B: VmBuilder> SizedVariable<B> for Felt<B::F> {
    fn size_of() -> usize {
        1
    }
}

impl<B: VmBuilder> Expression<B> for Felt<B::F> {
    type Value = Felt<B::F>;

    fn assign(&self, value: Felt<B::F>, builder: &mut B) {
        builder.push(AsmInstruction::ADDI(value.0, self.0, B::F::zero()));
    }
}

impl<B: VmBuilder> FromConstant<B> for Felt<B::F> {
    type Constant = B::F;

    fn imm(&self, constant: Self::Constant, builder: &mut B) {
        builder.push(AsmInstruction::IMM(self.0, constant));
    }
}

impl<B: VmBuilder> AlgebraicVariable<B> for Felt<B::F> {
    type ArithConst = B::F;
    type ArithExpr = Symbolic<B::F>;

    fn one() -> Self::Constant {
        B::F::one()
    }

    fn zero() -> Self::Constant {
        B::F::zero()
    }
}

impl<B: VmBuilder> FieldVariable<B> for Felt<B::F> {
    type F = B::F;
    type FieldExpr = Symbolic<B::F>;
}

impl<F> Add for Felt<F> {
    type Output = Symbolic<F>;

    fn add(self, rhs: Self) -> Symbolic<F> {
        Symbolic::<F>::from(self) + rhs
    }
}

impl<F> Add<F> for Felt<F> {
    type Output = Symbolic<F>;

    fn add(self, rhs: F) -> Symbolic<F> {
        Symbolic::<F>::from(self) + rhs
    }
}

impl<F> Add<Symbolic<F>> for Felt<F> {
    type Output = Symbolic<F>;

    fn add(self, rhs: Symbolic<F>) -> Symbolic<F> {
        Symbolic::<F>::from(self) + rhs
    }
}

impl<F> Sub for Felt<F> {
    type Output = Symbolic<F>;

    fn sub(self, rhs: Self) -> Symbolic<F> {
        Symbolic::<F>::from(self) - rhs
    }
}

impl<F> Sub<Symbolic<F>> for Felt<F> {
    type Output = Symbolic<F>;

    fn sub(self, rhs: Symbolic<F>) -> Symbolic<F> {
        Symbolic::<F>::from(self) - rhs
    }
}

impl<F> Sub<F> for Felt<F> {
    type Output = Symbolic<F>;

    fn sub(self, rhs: F) -> Symbolic<F> {
        Symbolic::<F>::from(self) - rhs
    }
}

impl<F> Mul for Felt<F> {
    type Output = Symbolic<F>;

    fn mul(self, rhs: Self) -> Symbolic<F> {
        Symbolic::<F>::from(self) * rhs
    }
}

impl<F> Mul<F> for Felt<F> {
    type Output = Symbolic<F>;

    fn mul(self, rhs: F) -> Symbolic<F> {
        Symbolic::<F>::from(self) * rhs
    }
}

impl<F> Mul<Symbolic<F>> for Felt<F> {
    type Output = Symbolic<F>;

    fn mul(self, rhs: Symbolic<F>) -> Symbolic<F> {
        Symbolic::<F>::from(self) * rhs
    }
}

impl<F> Div for Felt<F> {
    type Output = Symbolic<F>;

    fn div(self, rhs: Self) -> Symbolic<F> {
        Symbolic::<F>::from(self) / rhs
    }
}

impl<F> Div<F> for Felt<F> {
    type Output = Symbolic<F>;

    fn div(self, rhs: F) -> Symbolic<F> {
        Symbolic::<F>::from(self) / rhs
    }
}

impl<F> Div<Symbolic<F>> for Felt<F> {
    type Output = Symbolic<F>;

    fn div(self, rhs: Symbolic<F>) -> Symbolic<F> {
        Symbolic::<F>::from(self) / rhs
    }
}

impl<F> Neg for Felt<F> {
    type Output = Symbolic<F>;

    fn neg(self) -> Symbolic<F> {
        -Symbolic::from(self)
    }
}