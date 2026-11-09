// Compiler-only calls against an independently obtained exact SDK.
#include <cstddef>
#include <type_traits>
#include "ccl/public/app/ipreset.h"
#include "ccl/public/base/variant.h"
#include "ccl/public/collections/iunknownlist.h"
#include "ccl/public/gui/idatatarget.h"
#include "ccl/public/storage/iattributelist.h"
#include "ccl/public/storage/iurl.h"
#include "ccl/public/text/cstring.h"
#include "ccl/public/text/istring.h"
#include "ccl/public/plugservices.h"
#include "ccl/public/systemservices.h"
#include "ccl/public/system/inativefilesystem.h"

static_assert(sizeof(CCL::tbool)==1 && sizeof(CCL::tresult)==4);
static_assert(sizeof(CCL::UIDBytes)==16 && alignof(CCL::UIDBytes)==4);
static_assert(sizeof(CCL::PlainString)==16 && alignof(CCL::PlainString)==8);
static_assert(offsetof(CCL::PlainString,theString)==0 && offsetof(CCL::PlainString,__private)==8);
static_assert(sizeof(CCL::PlainCString)==16 && alignof(CCL::PlainCString)==8);
static_assert(offsetof(CCL::PlainCString,text)==0 && offsetof(CCL::PlainCString,theString)==8);
static_assert(sizeof(CCL::Variant)==16 && alignof(CCL::Variant)==8);
static_assert(offsetof(CCL::Variant,type)==0 && offsetof(CCL::Variant,lValue)==8);
static_assert(!std::is_trivially_destructible_v<CCL::Variant>);
extern "C" CCL::tresult abi_query(CCL::IUnknown* p,const CCL::UIDBytes* iid,void** out){return p->queryInterface(*iid,out);}
extern "C" unsigned abi_retain(CCL::IUnknown* p){return p->retain();}
extern "C" unsigned abi_release(CCL::IUnknown* p){return p->release();}
extern "C" CCL::tresult abi_create(CCL::IClassAllocator* p,const CCL::UIDBytes* cid,const CCL::UIDBytes* iid,void** out){return p->createInstance(*cid,*iid,out);}
extern "C" CCL::tbool abi_mediator_meta(CCL::IPresetMediator* p,CCL::IAttributeList* a){return p->getPresetMetaInfo(*a);}
extern "C" CCL::IUnknown* abi_mediator_target(CCL::IPresetMediator* p){return p->getPresetTarget();}
extern "C" CCL::IUnknownList* abi_presets(CCL::IPresetManager* p,CCL::IAttributeList* a){return p->getPresets(a,nullptr);}
extern "C" CCL::IUnknownIterator* abi_iterator(CCL::IContainer* p){return p->createIterator();}
extern "C" CCL::tbool abi_list_empty(CCL::IUnknownList* p){return p->isEmpty();}
extern "C" CCL::IUnknown* abi_list_first(CCL::IUnknownList* p){return p->getFirst();}
extern "C" CCL::IUnknown* abi_list_last(CCL::IUnknownList* p){return p->getLast();}
extern "C" CCL::tbool abi_list_contains(CCL::IUnknownList* p,CCL::IUnknown* o){return p->contains(o);}
extern "C" CCL::tbool abi_list_add(CCL::IUnknownList* p,CCL::IUnknown* o,CCL::tbool share){return p->add(o,share);}
extern "C" CCL::tbool abi_list_remove(CCL::IUnknownList* p,CCL::IUnknown* o){return p->remove(o);}
extern "C" void abi_list_clear(CCL::IUnknownList* p){p->removeAll();}
extern "C" CCL::tbool abi_iterator_done(CCL::IUnknownIterator* p){return p->done();}
extern "C" CCL::IUnknown* abi_iterator_next(CCL::IUnknownIterator* p){return p->nextUnknown();}
extern "C" CCL::tbool abi_read_only(CCL::IPreset* p){return p->isReadOnly();}
extern "C" const CCL::String* abi_name(CCL::IPreset* p){return &p->getPresetName();}
extern "C" CCL::IAttributeList* abi_meta(CCL::IPreset* p){return p->getMetaInfo();}
extern "C" CCL::tbool abi_url(CCL::IPreset* p,CCL::IUrl* u){return p->getUrl(*u);}
extern "C" void abi_url_clone(CCL::IUrl* p,CCL::IUrl** out){p->clone(*out);}
extern "C" CCL::tbool abi_url_equal(CCL::IUrl* p,CCL::IUrl* b){return p->isEqualUrl(*b,true);}
extern "C" CCL::tbool abi_url_empty(CCL::IUrl* p){return p->isEmpty();}
extern "C" int abi_url_kind(CCL::IUrl* p){return p->getType();}
extern "C" const CCL::String* abi_url_protocol(CCL::IUrl* p){return &p->getProtocol();}
extern "C" const CCL::String* abi_url_host(CCL::IUrl* p){return &p->getHostName();}
extern "C" CCL::tbool abi_url_native(CCL::IUrl* p){return p->isNativePath();}
extern "C" CCL::tbool abi_url_absolute(CCL::IUrl* p){return p->isAbsolute();}
extern "C" CCL::tbool abi_exists(CCL::IFileSystem* p,CCL::IUrl* u){return p->fileExists(*u);}
extern "C" CCL::tbool abi_attribute(CCL::IAttributeList* p,CCL::Variant* out,const CCL::CString* id){return p->getAttribute(*out,*id);}
extern "C" int abi_string_length(CCL::IString* p){return p->getLength();}
extern "C" CCL::tresult abi_string_copy(CCL::IString* p,CCL::uchar* out,int n){return p->copyTo(out,n);}
extern "C" CCL::tbool abi_can_insert(CCL::IDataTarget* p,CCL::IUnknownList* l){return p->canInsertData(*l,nullptr,nullptr,-1);}
extern "C" CCL::tbool abi_insert(CCL::IDataTarget* p,CCL::IUnknownList* l){return p->insertData(*l,nullptr,-1);}
extern "C" CCL::IPlugInManager* abi_allocator_export(){return &CCL::System::GetPlugInManager();}
extern "C" CCL::INativeFileSystem* abi_filesystem_export(){return &CCL::System::GetFileSystem();}
