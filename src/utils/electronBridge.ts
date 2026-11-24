type Rect = {x:number;y:number;width:number;height:number};
type StudioBridge = {
  chatOpen(owner:string):Promise<boolean>;
  chatBounds(owner:string,rect:Rect|null):Promise<boolean>;
  chatClose(owner:string):Promise<void>;
  chatExternal():Promise<void>;
  pdfStore(bytes:Uint8Array):Promise<string>;
  pdfRead(token:string):Promise<Uint8Array>;
  research(op:string,value?:unknown):Promise<unknown>;
  library(op:string,value?:unknown):Promise<unknown>;
  reader(op:string,value?:unknown):Promise<unknown>;
};
export function electronBridge(): StudioBridge|undefined {
  return (window as Window & {studio?:StudioBridge}).studio;
}
