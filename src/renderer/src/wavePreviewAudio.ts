/** Validate desktop/HTTP binary transport before handing WAV data to Web Audio. */
export function previewBytes(data: unknown): ArrayBuffer {
 let bytes:Uint8Array
 if(data instanceof ArrayBuffer)bytes=new Uint8Array(data)
 else if(ArrayBuffer.isView(data))bytes=new Uint8Array(data.buffer,data.byteOffset,data.byteLength)
 else if(Array.isArray(data)&&data.every(v=>Number.isInteger(v)&&v>=0&&v<=255))bytes=new Uint8Array(data)
 else throw new Error('Preview audio did not arrive correctly. Retry playback; if it persists, reopen the project.')
 if(bytes.length<44||bytes.length>4*1024*1024)throw new Error('Preview returned an empty or oversized audio file.')
 return bytes.slice().buffer
}
/** Decode bounded PCM/float WAV explicitly, avoiding WebKit codec differences. */
export function decodePreview(audio:AudioContext,data:unknown,expectedMs:number):AudioBuffer {
 const raw=previewBytes(data),v=new DataView(raw),tag=(at:number)=>String.fromCharCode(...new Uint8Array(raw,at,4))
 if(tag(0)!=='RIFF'||tag(8)!=='WAVE')throw new Error('Preview returned invalid WAV audio. Retry playback.')
 let format=0,channels=0,rate=0,bits=0,offset=0,length=0
 for(let i=12;i+8<=raw.byteLength;){const size=v.getUint32(i+4,true),body=i+8;if(body+size>raw.byteLength)throw new Error('Preview audio was cut off during transfer. Retry playback.');
  if(tag(i)==='fmt '&&size>=16){format=v.getUint16(body,true);channels=v.getUint16(body+2,true);rate=v.getUint32(body+4,true);bits=v.getUint16(body+14,true)}
  if(tag(i)==='data'){offset=body;length=size} i=body+size+(size%2)
 }
 if(!offset||!length||channels<1||channels>2||rate!==48000||!((format===1&&bits===16)||(format===3&&bits===32)))throw new Error('Preview audio uses an unsupported WAV format. Reopen the project and retry.')
 const frames=length/(channels*bits/8)
 if(!Number.isInteger(frames)||Math.abs(frames/rate*1000-expectedMs)>5)throw new Error('Preview audio has the wrong duration. Retry playback.')
 const buffer=audio.createBuffer(channels,frames,rate)
 for(let channel=0;channel<channels;channel++){const samples=buffer.getChannelData(channel);for(let i=0;i<frames;i++){const at=offset+(i*channels+channel)*bits/8;const value=format===1?v.getInt16(at,true)/32768:v.getFloat32(at,true);if(!Number.isFinite(value))throw new Error('Preview audio contains invalid samples.');samples[i]=value}}
 return buffer
}
