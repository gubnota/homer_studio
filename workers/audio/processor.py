"""One-request, versioned NDJSON processor. ML imports are lazy and stdout is framed."""
import contextlib
import importlib.util
import importlib.metadata
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

PROTOCOL = 1
ENGINES = {'passthrough', 'seed_vc', 'rvc', 'deepfilternet', 'resemble_enhance'}
MODULES = {'seed_vc': ['torch','torchaudio','librosa','transformers','yaml','munch','einops','funasr','modelscope'], 'rvc': ['torch','torchaudio','librosa','transformers','faiss','parselmouth','pyworld','av','soundfile','dotenv','local_attention'], 'deepfilternet': ['torch','torchaudio','df','libdf'], 'resemble_enhance': ['torch','torchaudio','resemble_enhance']}

def emit(kind, **data):
    sys.__stdout__.write(json.dumps({'protocol': PROTOCOL, 'type': kind, **data}) + '\n')
    sys.__stdout__.flush()

def model_files(engine, root):
    if engine == 'seed_vc': return ['inference.py', 'checkpoints/DiT_seed_v2_uvit_whisper_small_wavenet_bigvgan_pruned.pth', 'checkpoints/config_dit_mel_seed_uvit_whisper_small_wavenet.yml']
    if engine == 'rvc': return ['infer/cli.py', 'assets/hubert/hubert_base.pt', 'assets/rmvpe/rmvpe.pt']
    if engine == 'deepfilternet': return ['config.ini', 'checkpoints']
    if engine == 'resemble_enhance': return ['hparams.yaml', 'ds/G/default/mp_rank_00_model_states.pt']
    return []

def status(config):
    engine = config['engine']
    if engine not in ENGINES: raise ValueError('Unsupported audio engine')
    root = Path(config.get('modelDir') or '')
    def populated(path):
        return any(p.is_file() and p.stat().st_size > 0 for p in path.rglob('*')) if path.is_dir() else path.is_file() and path.stat().st_size > 0
    missing = [f for f in model_files(engine, root) if not populated(root / f)]
    absent = [module for module in MODULES.get(engine, []) if importlib.util.find_spec(module) is None]
    installed = not absent
    missing += ['Python module: ' + module for module in absent]
    if engine == 'seed_vc' and not populated(root / 'checkpoints/hf_cache'): missing.append('checkpoints/hf_cache (local Whisper, BigVGAN and CAMPPlus model cache)')
    backend = 'cpu'
    if installed and engine != 'passthrough':
        import torch
        if config.get('backend') in ('auto', 'mps') and engine in ('seed_vc', 'resemble_enhance') and torch.backends.mps.is_available(): backend = 'mps'
        if config.get('backend') == 'mps' and backend != 'mps': missing.append('MPS support for this engine')
    ready = installed and not missing
    return dict(engine=engine, installed=installed, ready=ready, backend=backend, message='Ready' if ready else 'Select the isolated Python environment and local model files.', missingFiles=missing, capabilities=['convert'] if engine in ('seed_vc', 'rvc') else ['denoise', 'enhance'])

def number(params, key, default, low, high, integer=False):
    value = params.get(key, default)
    if isinstance(value, bool) or not isinstance(value, (float, int)) or not low <= value <= high: raise ValueError(f'{key} must be between {low} and {high}')
    return int(value) if integer else float(value)

def process(request):
    config = request['config']; engine = config['engine']; params = request.get('params', {})
    info = status(config)
    if not info['ready']: raise ValueError(info['message'] + ' Missing: ' + ', '.join(info['missingFiles']))
    source = Path(request['sourcePath']).resolve(strict=True); output = Path(request['outputPath']).resolve(); root = Path(config.get('modelDir') or '.').resolve()
    if source == output: raise ValueError('Output must be a new file')
    if not isinstance(params, dict): raise ValueError('Parameters must be an object')
    output.parent.mkdir(parents=True, exist_ok=True)
    emit('stage', message='Loading local models', backend=info['backend'])
    # Disable Hugging Face/Transformers implicit model fetching during every inference.
    os.environ.update(HF_HUB_OFFLINE='1', TRANSFORMERS_OFFLINE='1', HF_DATASETS_OFFLINE='1')
    if engine == 'passthrough': shutil.copyfile(source, output)
    elif engine == 'seed_vc':
        reference = Path(request['referencePath']).resolve(strict=True)
        with tempfile.TemporaryDirectory(prefix='seed-', dir=output.parent) as folder:
            args = [sys.executable, str(Path(__file__).with_name('inference_driver.py')), info['backend'], str(root / 'inference.py'), '--source', str(source), '--target', str(reference), '--output', folder, '--checkpoint', str(root / model_files(engine, root)[1]), '--config', str(root / model_files(engine, root)[2]), '--diffusion-steps', str(number(params, 'diffusionSteps', 30, 1, 200, True)), '--length-adjust', str(number(params, 'lengthAdjust', 1, .5, 2)), '--inference-cfg-rate', str(number(params, 'guidance', .7, 0, 2)), '--fp16', 'False']
            environment = dict(os.environ)
            if info['backend'] == 'cpu': environment['PYTORCH_ENABLE_MPS_FALLBACK'] = '1'
            subprocess.run(args, cwd=root, env=environment, check=True, stdout=sys.stderr, stderr=sys.stderr)
            candidates = list(Path(folder).glob('*.wav'))
            if len(candidates) != 1: raise ValueError('Seed-VC did not return exactly one output WAV')
            shutil.move(str(candidates[0]), output)
    elif engine == 'rvc':
        model = Path(request['modelPath']).resolve(strict=True)
        args = [sys.executable, str(Path(__file__).with_name('inference_driver.py')), 'cpu', str(root / 'infer/cli.py'), '--model', str(model), '--input', str(source), '--output', str(output), '--pitch', str(number(params,'pitch',0,-24,24,True)), '--f0-method', 'rmvpe', '--index-rate', str(number(params,'indexRate',0,0,1)), '--protect', str(number(params,'protect',.33,0,.5)), '--overwrite']
        if request.get('indexPath'): args += ['--index', str(Path(request['indexPath']).resolve(strict=True))]
        subprocess.run(args, cwd=root, check=True, stdout=sys.stderr, stderr=sys.stderr)
    else:
        import torch
        import torchaudio
        wave, rate = torchaudio.load(str(source)); wave = wave.mean(dim=0)
        if engine == 'deepfilternet':
            from df.enhance import init_df, enhance
            model, state, *_ = init_df(model_base_dir=str(root), log_file=None)
            model = model.to('cpu')
            wave = torchaudio.functional.resample(wave,rate,state.sr()).unsqueeze(0)
            result = enhance(model,state,wave,pad=True,atten_lim_db=number(params,'attenuation',12,0,100)); rate = state.sr()
        else:
            from resemble_enhance.enhancer.inference import enhance, denoise
            if request['operation'] == 'denoise': result,rate = denoise(wave,rate,info['backend'],run_dir=str(root))
            else: result,rate = enhance(wave,rate,info['backend'],nfe=number(params,'nfe',32,1,128,True),solver='midpoint',lambd=number(params,'lambda',.5,0,1),tau=number(params,'tau',.5,0,1),run_dir=str(root))
        if result.ndim == 1: result = result.unsqueeze(0)
        if not torch.isfinite(result).all(): raise ValueError('Processor returned invalid audio samples')
        torchaudio.save(str(output),result.cpu(),rate,encoding='PCM_F',bits_per_sample=32)
    if not output.is_file() or output.stat().st_size < 44: raise ValueError('Processor returned no audio')
    return {'outputPath':str(output), 'backend':info['backend'], 'version':version(engine)}

def version(engine):
    package = {'deepfilternet':'DeepFilterNet','resemble_enhance':'resemble-enhance','seed_vc':'torch','rvc':'torch'}.get(engine)
    try: return 'homer-adapter-1 / ' + package + ' ' + importlib.metadata.version(package) if package else 'homer-adapter-1'
    except importlib.metadata.PackageNotFoundError: return 'homer-adapter-1'

def main():
    try:
        line = sys.stdin.buffer.readline(1024 * 1024 + 1)
        if len(line) > 1024 * 1024: raise ValueError('Request exceeds protocol limit')
        request = json.loads(line)
        if not isinstance(request, dict) or request.get('protocol') != PROTOCOL: raise ValueError('Unsupported protocol version')
        if request.get('action') not in ('status', 'process'): raise ValueError('Unsupported action')
        if not isinstance(request.get('config'), dict): raise ValueError('Configuration must be an object')
        with contextlib.redirect_stdout(sys.stderr):
            result = status(request['config']) if request.get('action') == 'status' else process(request)
        emit('result', result=result)
    except Exception as error:
        emit('error', code='PROCESSOR_FAILED', message=str(error)[:2000]); return 1
    return 0
if __name__ == '__main__': sys.exit(main())
