use p3_baby_bear::BabyBear;
use p3_challenger::DuplexChallenger;
use p3_commit::TwoAdicMultiplicativeCoset;
use p3_field::TwoAdicField;
use p3_field::{AbstractExtensionField, AbstractField};
use sp1_core::air::{MachineAir, Word, PV_DIGEST_NUM_WORDS};
use sp1_core::stark::{
    AirOpenedValues, ChipOpenedValues, Com, ShardCommitment, ShardOpenedValues, ShardProof,
};
use sp1_core::stark::{StarkGenericConfig, StarkVerifyingKey};
use sp1_core::utils::{
    BabyBearPoseidon2, BabyBearPoseidon2Inner, InnerChallenge, InnerDigest, InnerDigestHash,
    InnerPcsProof, InnerPerm, InnerVal,
};
use sp1_recursion_compiler::{
    config::InnerConfig,
    ir::{Array, Builder, Config, Ext, Felt, MemVariable, Var},
};
use sp1_recursion_core::air::Block;
use sp1_recursion_core::runtime::PERMUTATION_WIDTH;

use crate::challenger::DuplexChallengerVariable;
use crate::fri::TwoAdicMultiplicativeCosetVariable;
use crate::reduce::{
    SP1DeferredMemoryLayout, SP1DeferredMemoryLayoutVariable, SP1RecursionMemoryLayout,
    SP1RecursionMemoryLayoutVariable, SP1ReduceMemoryLayout, SP1ReduceMemoryLayoutVariable,
    SP1RootMemoryLayout, SP1RootMemoryLayoutVariable,
};
use crate::types::{
    AirOpenedValuesVariable, ChipOpenedValuesVariable, Sha256DigestVariable,
    ShardCommitmentVariable, ShardOpenedValuesVariable, ShardProofVariable, VerifyingKeyVariable,
};
use crate::types::{QuotientData, QuotientDataValues};
use crate::utils::{get_chip_quotient_data, get_preprocessed_data, get_sorted_indices};

pub trait Hintable<C: Config> {
    type HintVariable: MemVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable;

    fn write(&self) -> Vec<Vec<Block<C::F>>>;

    fn witness(variable: &Self::HintVariable, builder: &mut Builder<C>) {
        let target = Self::read(builder);
        builder.assign(variable.clone(), target);
    }
}

type C = InnerConfig;

impl Hintable<C> for usize {
    type HintVariable = Var<InnerVal>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_var()
    }

    fn write(&self) -> Vec<Vec<Block<InnerVal>>> {
        vec![vec![Block::from(InnerVal::from_canonical_usize(*self))]]
    }
}

impl Hintable<C> for InnerVal {
    type HintVariable = Felt<InnerVal>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_felt()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![vec![Block::from(*self)]]
    }
}

impl Hintable<C> for InnerChallenge {
    type HintVariable = Ext<InnerVal, InnerChallenge>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_ext()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![vec![Block::from((*self).as_base_slice())]]
    }
}

impl Hintable<C> for [Word<BabyBear>; PV_DIGEST_NUM_WORDS] {
    type HintVariable = Sha256DigestVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let bytes = builder.hint_felts();
        Sha256DigestVariable { bytes }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![self
            .iter()
            .flat_map(|w| w.0.iter().map(|f| Block::from(*f)))
            .collect::<Vec<_>>()]
    }
}

impl Hintable<C> for QuotientDataValues {
    type HintVariable = QuotientData<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let log_quotient_degree = usize::read(builder);
        let quotient_size = usize::read(builder);

        QuotientData {
            log_quotient_degree,
            quotient_size,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut buffer = Vec::new();
        buffer.extend(usize::write(&self.log_quotient_degree));
        buffer.extend(usize::write(&self.quotient_size));

        buffer
    }
}

impl Hintable<C> for TwoAdicMultiplicativeCoset<InnerVal> {
    type HintVariable = TwoAdicMultiplicativeCosetVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let log_n = usize::read(builder);
        let shift = InnerVal::read(builder);
        let g_val = InnerVal::read(builder);
        let size = usize::read(builder);

        // Initialize a domain.
        TwoAdicMultiplicativeCosetVariable::<C> {
            log_n,
            size,
            shift,
            g: g_val,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut vec = Vec::new();
        vec.extend(usize::write(&self.log_n));
        vec.extend(InnerVal::write(&self.shift));
        vec.extend(InnerVal::write(&InnerVal::two_adic_generator(self.log_n)));
        vec.extend(usize::write(&(1usize << (self.log_n))));
        vec
    }
}

trait VecAutoHintable<C: Config>: Hintable<C> {}

impl VecAutoHintable<C> for ShardProof<BabyBearPoseidon2> {}
impl VecAutoHintable<C> for ShardProof<BabyBearPoseidon2Inner> {}
impl VecAutoHintable<C> for TwoAdicMultiplicativeCoset<InnerVal> {}
impl VecAutoHintable<C> for Vec<usize> {}
impl VecAutoHintable<C> for QuotientDataValues {}
impl VecAutoHintable<C> for Vec<QuotientDataValues> {}

impl<I: VecAutoHintable<C>> VecAutoHintable<C> for &I {}

impl<H: Hintable<C>> Hintable<C> for &H {
    type HintVariable = H::HintVariable;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        H::read(builder)
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        H::write(self)
    }
}

impl<I: VecAutoHintable<C>> Hintable<C> for Vec<I> {
    type HintVariable = Array<C, I::HintVariable>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let len = builder.hint_var();
        let mut arr = builder.dyn_array(len);
        builder.range(0, len).for_each(|i, builder| {
            let hint = I::read(builder);
            builder.set(&mut arr, i, hint);
        });
        arr
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let len = InnerVal::from_canonical_usize(self.len());
        stream.push(vec![len.into()]);

        self.iter().for_each(|i| {
            let comm = I::write(i);
            stream.extend(comm);
        });

        stream
    }
}

impl Hintable<C> for Vec<usize> {
    type HintVariable = Array<C, Var<InnerVal>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_vars()
    }

    fn write(&self) -> Vec<Vec<Block<InnerVal>>> {
        vec![self
            .iter()
            .map(|x| Block::from(InnerVal::from_canonical_usize(*x)))
            .collect()]
    }
}

impl Hintable<C> for Vec<InnerVal> {
    type HintVariable = Array<C, Felt<InnerVal>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_felts()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![self.iter().map(|x| Block::from(*x)).collect()]
    }
}

impl Hintable<C> for Vec<InnerChallenge> {
    type HintVariable = Array<C, Ext<InnerVal, InnerChallenge>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_exts()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![self
            .iter()
            .map(|x| Block::from((*x).as_base_slice()))
            .collect()]
    }
}

impl Hintable<C> for AirOpenedValues<InnerChallenge> {
    type HintVariable = AirOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let local = Vec::<InnerChallenge>::read(builder);
        let next = Vec::<InnerChallenge>::read(builder);
        AirOpenedValuesVariable { local, next }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.local.write());
        stream.extend(self.next.write());
        stream
    }
}

impl Hintable<C> for Vec<Vec<InnerChallenge>> {
    type HintVariable = Array<C, Array<C, Ext<InnerVal, InnerChallenge>>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let len = builder.hint_var();
        let mut arr = builder.dyn_array(len);
        builder.range(0, len).for_each(|i, builder| {
            let hint = Vec::<InnerChallenge>::read(builder);
            builder.set(&mut arr, i, hint);
        });
        arr
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let len = InnerVal::from_canonical_usize(self.len());
        stream.push(vec![len.into()]);

        self.iter().for_each(|arr| {
            let comm = Vec::<InnerChallenge>::write(arr);
            stream.extend(comm);
        });

        stream
    }
}

impl Hintable<C> for ChipOpenedValues<InnerChallenge> {
    type HintVariable = ChipOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let preprocessed = AirOpenedValues::<InnerChallenge>::read(builder);
        let main = AirOpenedValues::<InnerChallenge>::read(builder);
        let permutation = AirOpenedValues::<InnerChallenge>::read(builder);
        let quotient = Vec::<Vec<InnerChallenge>>::read(builder);
        let cumulative_sum = InnerChallenge::read(builder);
        let log_degree = builder.hint_var();
        ChipOpenedValuesVariable {
            preprocessed,
            main,
            permutation,
            quotient,
            cumulative_sum,
            log_degree,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.preprocessed.write());
        stream.extend(self.main.write());
        stream.extend(self.permutation.write());
        stream.extend(self.quotient.write());
        stream.extend(self.cumulative_sum.write());
        stream.extend(self.log_degree.write());
        stream
    }
}

impl Hintable<C> for Vec<ChipOpenedValues<InnerChallenge>> {
    type HintVariable = Array<C, ChipOpenedValuesVariable<C>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let len = builder.hint_var();
        let mut arr = builder.dyn_array(len);
        builder.range(0, len).for_each(|i, builder| {
            let hint = ChipOpenedValues::<InnerChallenge>::read(builder);
            builder.set(&mut arr, i, hint);
        });
        arr
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let len = InnerVal::from_canonical_usize(self.len());
        stream.push(vec![len.into()]);

        self.iter().for_each(|arr| {
            let comm = ChipOpenedValues::<InnerChallenge>::write(arr);
            stream.extend(comm);
        });

        stream
    }
}

impl Hintable<C> for ShardOpenedValues<InnerChallenge> {
    type HintVariable = ShardOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let chips = Vec::<ChipOpenedValues<InnerChallenge>>::read(builder);
        ShardOpenedValuesVariable { chips }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.chips.write());
        stream
    }
}

impl Hintable<C> for ShardCommitment<InnerDigestHash> {
    type HintVariable = ShardCommitmentVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let main_commit = InnerDigest::read(builder);
        let permutation_commit = InnerDigest::read(builder);
        let quotient_commit = InnerDigest::read(builder);
        ShardCommitmentVariable {
            main_commit,
            permutation_commit,
            quotient_commit,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        let h: InnerDigest = self.main_commit.into();
        stream.extend(h.write());
        let h: InnerDigest = self.permutation_commit.into();
        stream.extend(h.write());
        let h: InnerDigest = self.quotient_commit.into();
        stream.extend(h.write());
        stream
    }
}

impl Hintable<C> for DuplexChallenger<InnerVal, InnerPerm, 16> {
    type HintVariable = DuplexChallengerVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let sponge_state = builder.hint_felts();
        let nb_inputs = builder.hint_var();
        let input_buffer = builder.hint_felts();
        let nb_outputs = builder.hint_var();
        let output_buffer = builder.hint_felts();
        DuplexChallengerVariable {
            sponge_state,
            nb_inputs,
            input_buffer,
            nb_outputs,
            output_buffer,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.sponge_state.to_vec().write());
        stream.extend(self.input_buffer.len().write());
        let mut input_padded = self.input_buffer.to_vec();
        input_padded.resize(PERMUTATION_WIDTH, InnerVal::zero());
        stream.extend(input_padded.write());
        stream.extend(self.output_buffer.len().write());
        let mut output_padded = self.output_buffer.to_vec();
        output_padded.resize(PERMUTATION_WIDTH, InnerVal::zero());
        stream.extend(output_padded.write());
        stream
    }
}

impl<
        SC: StarkGenericConfig<
            Pcs = <BabyBearPoseidon2 as StarkGenericConfig>::Pcs,
            Challenge = <BabyBearPoseidon2 as StarkGenericConfig>::Challenge,
            Challenger = <BabyBearPoseidon2 as StarkGenericConfig>::Challenger,
        >,
    > Hintable<C> for StarkVerifyingKey<SC>
{
    type HintVariable = VerifyingKeyVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let commitment = InnerDigest::read(builder);
        let pc_start = InnerVal::read(builder);
        VerifyingKeyVariable {
            commitment,
            pc_start,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        let h: InnerDigest = self.commit.into();
        stream.extend(h.write());
        stream.extend(self.pc_start.write());
        stream
    }
}

// Implement Hintable<C> for ShardProof where SC is equivalent to BabyBearPoseidon2
impl<
        SC: StarkGenericConfig<
            Pcs = <BabyBearPoseidon2 as StarkGenericConfig>::Pcs,
            Challenge = <BabyBearPoseidon2 as StarkGenericConfig>::Challenge,
            Challenger = <BabyBearPoseidon2 as StarkGenericConfig>::Challenger,
        >,
    > Hintable<C> for ShardProof<SC>
where
    ShardCommitment<Com<SC>>: Hintable<C>,
{
    type HintVariable = ShardProofVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let commitment = ShardCommitment::read(builder);
        let opened_values = ShardOpenedValues::read(builder);
        let opening_proof = InnerPcsProof::read(builder);
        let public_values = Vec::<InnerVal>::read(builder);
        ShardProofVariable {
            commitment,
            opened_values,
            opening_proof,
            public_values,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.commitment.write());
        stream.extend(self.opened_values.write());
        stream.extend(self.opening_proof.write());
        stream.extend(self.public_values.write());

        stream
    }
}

impl<'a, A: MachineAir<BabyBear>> Hintable<C>
    for SP1RecursionMemoryLayout<'a, BabyBearPoseidon2, A>
{
    type HintVariable = SP1RecursionMemoryLayoutVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let vk = StarkVerifyingKey::<BabyBearPoseidon2>::read(builder);
        let shard_proofs = Vec::<ShardProof<BabyBearPoseidon2>>::read(builder);
        let shard_chip_quotient_data = Vec::<Vec<QuotientDataValues>>::read(builder);
        let shard_sorted_indices = Vec::<Vec<usize>>::read(builder);
        let preprocessed_sorted_idxs = Vec::<usize>::read(builder);
        let prep_domains = Vec::<TwoAdicMultiplicativeCoset<InnerVal>>::read(builder);
        let leaf_challenger = DuplexChallenger::<InnerVal, InnerPerm, 16>::read(builder);
        let initial_reconstruct_challenger =
            DuplexChallenger::<InnerVal, InnerPerm, 16>::read(builder);
        let is_complete = builder.hint_var();

        SP1RecursionMemoryLayoutVariable {
            vk,
            shard_proofs,
            shard_chip_quotient_data,
            shard_sorted_indices,
            preprocessed_sorted_idxs,
            prep_domains,
            leaf_challenger,
            initial_reconstruct_challenger,
            is_complete,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let (prep_sorted_indices, prep_domains) =
            get_preprocessed_data::<BabyBearPoseidon2, A>(self.machine, self.vk);

        let shard_chip_quotient_data = self
            .shard_proofs
            .iter()
            .map(|proof| get_chip_quotient_data::<BabyBearPoseidon2, A>(self.machine, proof))
            .collect::<Vec<_>>();

        let shard_sorted_indices = self
            .shard_proofs
            .iter()
            .map(|proof| get_sorted_indices::<BabyBearPoseidon2, A>(self.machine, proof))
            .collect::<Vec<_>>();

        stream.extend(self.vk.write());
        stream.extend(self.shard_proofs.write());
        stream.extend(shard_chip_quotient_data.write());
        stream.extend(shard_sorted_indices.write());
        stream.extend(prep_sorted_indices.write());
        stream.extend(prep_domains.write());
        stream.extend(self.leaf_challenger.write());
        stream.extend(self.initial_reconstruct_challenger.write());
        stream.extend((self.is_complete as usize).write());

        stream
    }
}

impl<'a, A: MachineAir<BabyBear>> Hintable<C> for SP1ReduceMemoryLayout<'a, BabyBearPoseidon2, A> {
    type HintVariable = SP1ReduceMemoryLayoutVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let reduce_vk = StarkVerifyingKey::<BabyBearPoseidon2>::read(builder);
        let reduce_prep_sorted_idxs = Vec::<usize>::read(builder);
        let reduce_prep_domains = Vec::<TwoAdicMultiplicativeCoset<InnerVal>>::read(builder);
        let shard_proofs = Vec::<ShardProof<BabyBearPoseidon2>>::read(builder);
        let shard_chip_quotient_data = Vec::<Vec<QuotientDataValues>>::read(builder);
        let shard_sorted_indices = Vec::<Vec<usize>>::read(builder);
        let kinds = Vec::<usize>::read(builder);
        let is_complete = builder.hint_var();

        SP1ReduceMemoryLayoutVariable {
            reduce_vk,
            reduce_prep_sorted_idxs,
            reduce_prep_domains,
            shard_proofs,
            shard_chip_quotient_data,
            shard_sorted_indices,
            kinds,
            is_complete,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let (reduce_prep_sorted_idxs, reduce_prep_domains) =
            get_preprocessed_data::<BabyBearPoseidon2, _>(self.recursive_machine, self.reduce_vk);

        let shard_chip_quotient_data = self
            .shard_proofs
            .iter()
            .map(|proof| {
                get_chip_quotient_data::<BabyBearPoseidon2, A>(self.recursive_machine, proof)
            })
            .collect::<Vec<_>>();

        let shard_sorted_indices = self
            .shard_proofs
            .iter()
            .map(|proof| get_sorted_indices::<BabyBearPoseidon2, A>(self.recursive_machine, proof))
            .collect::<Vec<_>>();

        let kinds = self.kinds.iter().map(|k| *k as usize).collect::<Vec<_>>();

        stream.extend(self.reduce_vk.write());
        stream.extend(reduce_prep_sorted_idxs.write());
        stream.extend(reduce_prep_domains.write());
        stream.extend(self.shard_proofs.write());
        stream.extend(shard_chip_quotient_data.write());
        stream.extend(shard_sorted_indices.write());
        stream.extend(kinds.write());
        stream.extend((self.is_complete as usize).write());

        stream
    }
}

impl<'a, A: MachineAir<BabyBear>> Hintable<C> for SP1RootMemoryLayout<'a, BabyBearPoseidon2, A> {
    type HintVariable = SP1RootMemoryLayoutVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let proof = ShardProof::<BabyBearPoseidon2>::read(builder);
        let chip_quotient_data = Vec::<QuotientDataValues>::read(builder);
        let sorted_indices = Vec::<usize>::read(builder);
        let is_reduce = builder.hint_var();

        SP1RootMemoryLayoutVariable {
            proof,
            chip_quotient_data,
            sorted_indices,
            is_reduce,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let chip_quotient_data =
            get_chip_quotient_data::<BabyBearPoseidon2, A>(self.machine, &self.proof);

        let sorted_indices = get_sorted_indices::<BabyBearPoseidon2, A>(self.machine, &self.proof);

        stream.extend(self.proof.write());
        stream.extend(chip_quotient_data.write());
        stream.extend(sorted_indices.write());
        stream.extend((self.is_reduce as usize).write());

        stream
    }
}

impl<'a, A: MachineAir<BabyBear>> Hintable<C>
    for SP1DeferredMemoryLayout<'a, BabyBearPoseidon2, A>
{
    type HintVariable = SP1DeferredMemoryLayoutVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let reduce_vk = StarkVerifyingKey::<BabyBearPoseidon2>::read(builder);
        let reduce_prep_sorted_idxs = Vec::<usize>::read(builder);
        let reduce_prep_domains = Vec::<TwoAdicMultiplicativeCoset<InnerVal>>::read(builder);
        let proofs = Vec::<ShardProof<BabyBearPoseidon2>>::read(builder);
        let proof_chip_quotient_data = Vec::<Vec<QuotientDataValues>>::read(builder);
        let proof_sorted_indices = Vec::<Vec<usize>>::read(builder);
        let start_reconstruct_deferred_digest = Vec::<BabyBear>::read(builder);
        let is_complete = builder.hint_var();

        SP1DeferredMemoryLayoutVariable {
            reduce_vk,
            reduce_prep_sorted_idxs,
            reduce_prep_domains,
            proofs,
            proof_chip_quotient_data,
            proof_sorted_indices,
            start_reconstruct_deferred_digest,
            is_complete,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let (reduce_prep_sorted_idxs, reduce_prep_domains) =
            get_preprocessed_data::<BabyBearPoseidon2, _>(self.machine, self.reduce_vk);

        let shard_chip_quotient_data = self
            .proofs
            .iter()
            .map(|proof| get_chip_quotient_data::<BabyBearPoseidon2, A>(self.machine, proof))
            .collect::<Vec<_>>();

        let shard_sorted_indices = self
            .proofs
            .iter()
            .map(|proof| get_sorted_indices::<BabyBearPoseidon2, A>(self.machine, proof))
            .collect::<Vec<_>>();

        stream.extend(self.reduce_vk.write());
        stream.extend(reduce_prep_sorted_idxs.write());
        stream.extend(reduce_prep_domains.write());
        stream.extend(self.proofs.write());
        stream.extend(shard_chip_quotient_data.write());
        stream.extend(shard_sorted_indices.write());
        stream.extend(self.start_reconstruct_deferred_digest.write());
        stream.extend((self.is_complete as usize).write());

        stream
    }
}
