// Fixed internal validator runner. Caller supplies no executable or shell text.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const {workspaceDir,candidatePath,finalPath,receiptPath,policy,config}=JSON.parse(await fs.readFile(process.argv[2],'utf8'));
const {finalizePresentation}=await import(pathToFileURL(path.join(config.skillDir,'container_tools/artifact_tool_utils.mjs')).href);
await finalizePresentation({workspaceDir,candidatePath,finalPath,receiptPath,pythonExecutable:config.pythonExecutable,
  integrityValidatorPath:path.join(config.skillDir,'container_tools/inspect_presentation_package_integrity.py'),
  layoutValidatorPath:path.join(config.skillDir,'container_tools/inspect_presentation_layout_geometry.py'),
  layoutArgs:['--expected-slide-size-emu',policy.slideSizeEmu.join(','),'--validate-bullet-geometry','--validate-heading-fit'],
  explicitTotalSlideCount:policy.slideCount,requiredNativeTableOwnerSlides:[],requiredNativeChartOwnerSlides:[],
  ...(policy.families.length?{fontPolicy:{basis:'design',families:policy.families}}:{}),verifyArtifactToolImport:true});
