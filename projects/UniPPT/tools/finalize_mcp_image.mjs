import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const root=path.resolve('.');
// Cache versions can be replaced during an app update. Callers explicitly
// select the installed skill they have read, rather than silently skipping QA.
const skill=process.env.UNIPPT_PRESENTATION_SKILL_DIR;
if(!skill||!path.isAbsolute(skill))throw new Error('Configure an absolute UNIPPT_PRESENTATION_SKILL_DIR');
const runtime=JSON.parse(await fs.readFile(path.join(root,'.unippt-quality-runtime.json'),'utf8'));
process.env.RUNTIME_NODE_MODULES ||= runtime.nodeModules;
const pythonExecutable=process.env.UNIPPT_PYTHON||runtime.pythonExecutable;
if(!pythonExecutable)throw new Error('Configure UNIPPT_PYTHON or quality runtime pythonExecutable');
const candidatePath=path.resolve(process.argv[2]),finalPath=path.resolve(process.argv[3]);
const {finalizePresentation}=await import(pathToFileURL(path.join(skill,'container_tools/artifact_tool_utils.mjs')).href);
const receiptPath=path.resolve(process.argv[4]||path.join(root,'.tmp/mcp-image-clock-v2/finalization.json'));
await fs.mkdir(path.dirname(receiptPath),{recursive:true});
try{await fs.access(receiptPath);throw new Error('Receipt already exists; choose a fresh QA output path');}catch(error){if(error.code!=='ENOENT')throw error;}
try{
 const result=await finalizePresentation({workspaceDir:root,candidatePath,finalPath,pythonExecutable,integrityValidatorPath:path.join(skill,'container_tools/inspect_presentation_package_integrity.py'),layoutValidatorPath:path.join(skill,'container_tools/inspect_presentation_layout_geometry.py'),layoutArgs:['--expected-slide-size-emu','12192000,6858000','--validate-bullet-geometry','--validate-heading-fit'],explicitTotalSlideCount:1,requiredNativeTableOwnerSlides:[],requiredNativeChartOwnerSlides:[],fontPolicy:{basis:'design',families:['Times New Roman','Cambria Math']},verifyArtifactToolImport:true,receiptPath});
 console.log(JSON.stringify(result));
}catch(error){const failure={passed:false,candidatePath,error:error.message,note:'Exact native candidate, not reauthored or re-saved through Artifact Tool. This extra development QA is after the MCP clock closed.'};await fs.writeFile(receiptPath,JSON.stringify(failure,null,2),{flag:'wx'});console.log(JSON.stringify(failure));process.exitCode=1;}
