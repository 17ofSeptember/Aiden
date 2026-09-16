import {invoke} from '@tauri-apps/api/core';
export async function api<T>(operation:string,data:unknown=null):Promise<T>{return invoke<T>('request',{operation,data});}
export function download(name:string,data:unknown){const url=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
