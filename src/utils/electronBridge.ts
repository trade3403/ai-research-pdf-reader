type Rect = {x:number;y:number;width:number;height:number};
type StudioBridge = {
  chatOpen(owner:string):Promise<boolean>;
  chatBounds(owner:string,rect:Rect|null):Promise<boolean>;
  chatClose(owner:string):Promise<void>;
  chatExternal():Promise<void>;
  pdfStore(bytes:Uint8Array):Promise<string>;
  pdfRead(token:string):Promise<Uint8Array>;
  azureKey(op:'save'|'has'|'remove',key?:string):Promise<boolean|void>;
  azureTranslate(request:{texts:string[];region:string;from:string;to:string}):Promise<{translations:string[]}>;
  docxSave(name:string,bytes:Uint8Array):Promise<string|null>;
  backup(op:'create'|'verify'):Promise<{files:number;pdfs:number;bytes:number;externalReferences:number;path?:string}|null>;
  research(op:string,value?:unknown):Promise<unknown>;
  library(op:string,value?:unknown):Promise<unknown>;
  reader(op:string,value?:unknown):Promise<unknown>;
};
export function electronBridge(): StudioBridge|undefined {
  return (window as Window & {studio?:StudioBridge}).studio;
}
