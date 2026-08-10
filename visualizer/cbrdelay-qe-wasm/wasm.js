let instance;
const wasmUrl = './cbrdelay-qe-wasm/target/wasm32-unknown-unknown/release/syntra_cbrdelay_qe_wasm.wasm';
async function load(){
  if(!instance){const response=await fetch(wasmUrl);const bytes=await response.arrayBuffer();instance=(await WebAssembly.instantiate(bytes,{})).instance;}
  return instance.exports;
}
function put(memory, values){const offset=memory.buffer.byteLength;const pages=Math.ceil((offset+values.length*8-memory.buffer.byteLength)/65536);if(pages>0)memory.grow(pages);new Float64Array(memory.buffer,offset,values.length).set(values);return offset;}
export async function sampleCbrdelayBelief(steps,count=10){
  const w=await load(),a=steps.map(s=>s.arrival),s=steps.map(x=>x.service);
  // The editable curve is cumulative L.  The generated QE dispatcher expects
  // a vector of loss-observation events, not one entry per timestep.  Until
  // CCA logs provide explicit observation ages, a strictly increasing L value
  // is treated as a newly observed event.
  const l=[];let previous=steps[0]?.loss??0;for(const step of steps.slice(1)){if(step.loss>previous+.001)l.push(step.loss);previous=Math.max(previous,step.loss)}
  const memory=w.memory;
  const ap=put(memory,a),sp=put(memory,s),lp=put(memory,l);w.qe_sample(ap,sp,lp,a.length,l.length,count,Math.max(100,...a));
  const out=new Float64Array(memory.buffer,w.qe_output_ptr(),1+4096*3),n=out[0]|0,points=[];for(let i=0;i<n;i++)points.push([out[1+i*3],out[2+i*3],out[3+i*3]]);return points;
}
