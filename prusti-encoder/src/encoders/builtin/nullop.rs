use crate::encoders::{
    TyUsePureEnc,
    ty::{RustTy, RustTyDecomposition},
};
use prusti_rustc_interface::middle::mir;
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::FunctionIdn;

pub struct MirBuiltinNullOpEnc;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct MirBuiltinNullOpTask<'vir> {
    result_ty: RustTy<'vir>,
    op: mir::NullOp<'vir>,
    ty: RustTy<'vir>,
}

impl<'vir> MirBuiltinNullOpTask<'vir> {
    pub fn new(
        result_ty: RustTyDecomposition<'vir>,
        op: mir::NullOp<'vir>,
        ty: RustTyDecomposition<'vir>,
    ) -> Self {
        Self {
            result_ty: result_ty.ty,
            op,
            ty: ty.ty,
        }
    }
}

impl TaskEncoder for MirBuiltinNullOpEnc {
    task_encoder::encoder_cache!(MirBuiltinNullOpEnc);
    const ENCODER_NAME: &'static str = "MIR builtin nullary op encoder";

    type TaskDescription<'vir> = MirBuiltinNullOpTask<'vir>;

    type OutputFullDependency<'vir> = vir::FunctionIdn<'vir, (), vir::Snap>;
    type OutputFullLocal<'vir> = vir::Function<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }

    fn do_encode_full<'vir>(
        task_key: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task_key, ())?;
        let MirBuiltinNullOpTask { result_ty, op, ty } = *task_key;
        vir::with_vcx(|vcx| {
            let ty_task = RustTyDecomposition::identity(result_ty);
            let r_ty = deps.require_ref::<TyUsePureEnc>(ty_task)?;
            let name = format!("mir_nullop_{op:?}_{}", ty.name());
            let name = vir::vir_format_identifier!(vcx, "{name}");
            let fn_idn = FunctionIdn::new(name, (), r_ty.snapshot);
            let function = vcx.mk_function(fn_idn, (), &[], &[], None, None);
            Ok((function, fn_idn))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for function in Self::all_outputs_local_no_errors(program) {
            program.add_function(function);
        }
    }
}
