"""Run three comparable fixed-size world workloads and publish the numeric summary."""
import argparse,json,subprocess
from pathlib import Path
from datetime import datetime
from zoneinfo import ZoneInfo
parser=argparse.ArgumentParser()
parser.add_argument('--size',default='1280x720')
parser.add_argument('--frames',default='90')
parser.add_argument('--counts',default='100,500,1000,5000,10000')
parser.add_argument('--lights',default='128')
parser.add_argument('--output',default='web/measurements/world.json')
args=parser.parse_args()
root=Path(__file__).resolve().parent.parent
stamp=datetime.now(ZoneInfo('America/Los_Angeles')).strftime('%Y-%m-%d')
runs=[]
for name,lights,steps in [('base','0','0'),('lights',args.lights,'0'),('full',args.lights,'16')]:
    out=root/'benchmarks'/f'{stamp}-world-{args.size}-{name}.json'
    subprocess.run([str(root/'target/release/benchmark'),'--world','--scene','rain','--lights',lights,'--reflection-steps',steps,'--counts',args.counts,'--size',args.size,'--frames',args.frames,'--output',str(out)],cwd=root,check=True)
    data=json.loads(out.read_text());data['profile']=name;runs.append(data)
summary={'kind':'sprite-lab-world-comparison','date':stamp,'adapter':runs[0]['adapter']['name']+' / '+runs[0]['adapter']['backend'],'viewport':runs[0]['viewport'],'lights':int(args.lights),'reflectionSteps':16,'objectSize':48,'scene':'rain','runs':runs}
output=root/args.output;output.parent.mkdir(parents=True,exist_ok=True);output.write_text(json.dumps(summary,indent=2)+'\n')
print('Saved',output)
