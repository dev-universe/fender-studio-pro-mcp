// Private compiler-only API call probes. Never linked into the add-on or loaded.
// Independently authored; no CCL implementation code is copied here.
#include <cstddef>
#include <cstdio>
#include <new>
#include <type_traits>
#include <dlfcn.h>
#include <CoreFoundation/CoreFoundation.h>
#include "ccl/public/app/idocument.h"
#include "ccl/public/app/ipreset.h"
#include "ccl/public/base/iobjectnode.h"
#include "ccl/public/base/iobject.h"
#include "ccl/public/base/istream.h"
#include "ccl/public/gui/icontroller.h"
#include "ccl/public/gui/iparameter.h"
#include "ccl/public/gui/idatatarget.h"
#include "ccl/public/gui/framework/iuserinterface.h"
#include "ccl/public/gui/framework/itimer.h"
#include "ccl/public/gui/framework/iwindow.h"
#include "ccl/public/gui/framework/iview.h"
#include "ccl/public/plugins/iobjecttable.h"
#include "ccl/public/plugins/icomponent.h"
#include "ccl/public/storage/istorage.h"
#include "ccl/public/system/isignalhandler.h"
#include "ccl/public/system/ithreading.h"
#include "ccl/public/text/cstring.h"
#include "ccl/public/base/imessage.h"
#include "ccl/public/plugins/iscriptingmanager.h"
#include "ccl/public/plugservices.h"
#include "ccl/public/systemservices.h"
#include "ccl/public/system/inativefilesystem.h"

static_assert(sizeof(CCL::Variant) == 16);
static_assert(alignof(CCL::Variant) == 8);
static_assert(offsetof(CCL::Variant, type) == 0);
static_assert(offsetof(CCL::Variant, lValue) == 8);
static_assert(!std::is_trivially_copyable_v<CCL::Variant>);
static_assert(!std::is_trivially_destructible_v<CCL::Variant>);
static_assert(sizeof(CCL::PlainCString) == 16);
static_assert(alignof(CCL::PlainCString) == 8);
static_assert(offsetof(CCL::PlainCString, text) == 0);
static_assert(offsetof(CCL::PlainCString, theString) == 8);
static_assert(sizeof(CCL::MutableCString) == 16);
static_assert(sizeof(Core::Threads::ThreadID) == 8);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IStorable>);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IStream>);
static_assert(CCL::IStream::kSeekSet==0 && CCL::IStream::kSeekCur==1 && CCL::IStream::kSeekEnd==2);
static_assert(sizeof(CCL::IStream)==8 && sizeof(CCL::int64)==8);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IPresetMediator>);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IPresetMetaDataProvider>);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IDataTarget>);
static_assert(std::is_base_of_v<CCL::IUnknown,CCL::IPresetManager>);

extern "C" int abi_controller_count(CCL::IController* p) { return p->countParameters(); }
extern "C" CCL::IController* abi_controller_provider(CCL::IControllerProvider* p) { return p->getController(); }
extern "C" CCL::IParameter* abi_controller_parameter(CCL::IController* p, int i) { return p->getParameterAt(i); }
extern "C" int abi_parameter_type(CCL::IParameter* p) { return p->getType(); }
extern "C" const CCL::CString* abi_parameter_name(CCL::IParameter* p) { return &p->getName(); }
extern "C" int abi_parameter_tag(CCL::IParameter* p) { return p->getTag(); }
extern "C" CCL::IUnknown* abi_parameter_controller(CCL::IParameter* p) { return p->getController(); }
extern "C" float abi_parameter_normalized(CCL::IParameter* p) { return p->getNormalized(); }
// This explicit output placement exposes C++'s indirect result register in
// compiler output. A Rust repr(C) 16-byte return is not this calling convention.
extern "C" void abi_capture_parameter_value(CCL::IParameter* p, void* out) { new(out) CCL::Variant(p->getValue()); }
extern "C" const CCL::UIDBytes* abi_node_uid(CCL::IObjectNode* p) { return &p->getObjectUID(); }
extern "C" const CCL::UIDBytes* abi_node_class(CCL::IObjectNode* p) { return &p->getClassUID(); }
extern "C" CCL::IObjectNode* abi_node_find(CCL::IObjectNode* p, CCL::String* id) { return p->findChild(*id); }
extern "C" CCL::IUnknown* abi_object_named(CCL::IObjectTable* p, const CCL::CString* name) { return p->getObjectByName(*name); }
extern "C" CCL::IUnknown* abi_object_identified(CCL::IObjectTable* p, const CCL::UIDBytes* id) { return p->getObjectByID(*id); }
extern "C" CCL::tbool abi_object_property(CCL::IObject* p, CCL::Variant* out, const CCL::CString* name) { return p->getProperty(*out,*name); }
extern "C" CCL::IDocument* abi_document_active(CCL::IDocumentManager* p) { return p->getActiveIDocument(); }
extern "C" CCL::IUnknown* abi_document_model(CCL::IDocument* p) { return p->getModel(); }
extern "C" int abi_desktop_count(CCL::IDesktop* p) { return p->countWindows(); }
extern "C" CCL::IWindow* abi_desktop_window(CCL::IDesktop* p,int i) { return p->getWindow(i); }
extern "C" CCL::IUnknown* abi_window_controller(CCL::IWindow* p) { return p->getController(); }
extern "C" CCL::IUnknown* abi_view_controller(CCL::IView* p) { return p->getController(); }
extern "C" CCL::IViewChildren* abi_view_children(CCL::IView* p) { return &p->getChildren(); }
extern "C" CCL::IViewIterator* abi_children_iterator(CCL::IViewChildren* p) { return p->createIterator(); }
extern "C" CCL::tbool abi_iterator_done(CCL::IViewIterator* p) { return p->done(); }
extern "C" CCL::IView* abi_iterator_next(CCL::IViewIterator* p) { return p->next(); }
extern "C" CCL::IParameter* abi_control_parameter(CCL::IControl* p) { return p->getParameter(); }
extern "C" void abi_idle_add(CCL::IUserInterface* p, CCL::ITimerTask* task) { p->addIdleTask(task); }
extern "C" void abi_idle_remove(CCL::IUserInterface* p, CCL::ITimerTask* task) { p->removeIdleTask(task); }
extern "C" CCL::tresult abi_post(CCL::ISignalHandler* p, CCL::IObserver* o, CCL::IMessage* msg) { return p->postMessage(o,msg,0); }
extern "C" CCL::tresult abi_cancel_messages(CCL::ISignalHandler* p, CCL::IObserver* o) { return p->cancelMessages(o); }
extern "C" Core::Threads::ThreadID abi_thread_id(CCL::Threading::IThread* p) { return p->getThreadID(); }
extern "C" const CCL::String* abi_node_id(CCL::IObjectNode* p) { return &p->getObjectID(); }
extern "C" CCL::IObjectNode* abi_node_parent(CCL::IObjectNode* p) { return p->getParent(); }
extern "C" int abi_node_count(CCL::IObjectNode* p) { return p->countChildren(); }
extern "C" CCL::IObjectNode* abi_node_child(CCL::IObjectNode* p,int i) { return p->getChild(i); }
extern "C" CCL::IObjectNode* abi_node_lookup(CCL::IObjectNode* p,CCL::String* path) { return p->lookupChild(*path); }
extern "C" const CCL::ITypeInfo* abi_object_type(CCL::IObject* p) { return &p->getTypeInfo(); }
extern "C" const char* abi_type_name(CCL::ITypeInfo* p) { return p->getClassName(); }
extern "C" CCL::tresult abi_string_copy(CCL::IString* p,CCL::uchar* out,int size) { return p->copyTo(out,size); }
extern "C" int abi_structured_count(CCL::IStructuredParameter* p) { return p->countSubParameters(); }
extern "C" CCL::IUnknown* abi_alias_plugin(CCL::IComponentAlias* p) { return p->getPlugInUnknown(); }
extern "C" int abi_string_length(CCL::IString* p) { return p->getLength(); }

// Own declaration of a version-specific binary interoperability shape; not a
// copied vendor SDK type. Field semantics intentionally remain unknown except
// for the six fields independently established in the private evidence note.
struct ProbeParamInfo {
    unsigned tag, folder;
    CCL::PlainCString name, cstring24;
    CCL::PlainString title, string56, string72;
    CCL::PlainCString cstring88;
    float float104;
    unsigned integer108, flags, integer116;
    unsigned long long integer120;
};
static_assert(sizeof(ProbeParamInfo)==128 && alignof(ProbeParamInfo)==8);
static_assert(offsetof(ProbeParamInfo,tag)==0 && offsetof(ProbeParamInfo,folder)==4);
static_assert(offsetof(ProbeParamInfo,name)==8 && offsetof(ProbeParamInfo,cstring24)==24);
static_assert(offsetof(ProbeParamInfo,title)==40 && offsetof(ProbeParamInfo,string56)==56);
static_assert(offsetof(ProbeParamInfo,string72)==72 && offsetof(ProbeParamInfo,cstring88)==88);
static_assert(offsetof(ProbeParamInfo,float104)==104 && offsetof(ProbeParamInfo,integer108)==108);
static_assert(offsetof(ProbeParamInfo,flags)==112 && offsetof(ProbeParamInfo,integer116)==116);
static_assert(offsetof(ProbeParamInfo,integer120)==120);
struct ProbePrivateDSP: CCL::IUnknown {
    virtual int count() = 0;
    virtual CCL::tbool info(ProbeParamInfo& out,int index) = 0;
};
extern "C" int abi_private_count(ProbePrivateDSP* p) { return p->count(); }
extern "C" CCL::tbool abi_private_info(ProbePrivateDSP* p,ProbeParamInfo* out,int index) { return p->info(*out,index); }
static_assert(RTLD_NOLOAD==0x10 && RTLD_NODELETE==0x80);
static_assert(sizeof(CFIndex)==8 && sizeof(CFTypeID)==8 && sizeof(CFBundleRef)==8);
static_assert(sizeof(CCL::ITypeInfo::MethodDefinition)==24);
static_assert(offsetof(CCL::ITypeInfo::MethodDefinition,name)==0);
static_assert(offsetof(CCL::ITypeInfo::MethodDefinition,args)==8);
static_assert(offsetof(CCL::ITypeInfo::MethodDefinition,retval)==16);
static_assert(CCL::ITypeInfo::kScriptable==2);
static_assert(CCL::Variant::kString==3 && CCL::Variant::kObject==4 && CCL::Variant::kShared==16);
extern "C" CCL::tresult abi_table_register(CCL::IObjectTable* p,CCL::IUnknown* o,const CCL::UIDBytes* id,const CCL::CString* name) { return p->registerObject(o,*id,*name,0); }
extern "C" CCL::tresult abi_table_unregister(CCL::IObjectTable* p,CCL::IUnknown* o) { return p->unregisterObject(o); }
extern "C" const CCL::CString* abi_message_id(CCL::IMessage* p) { return &p->getID(); }
extern "C" int abi_message_count(CCL::IMessage* p) { return p->getArgCount(); }
extern "C" const CCL::Variant* abi_message_argument(CCL::IMessage* p,int i) { return &p->getArg(i); }
extern "C" const CCL::Variant* abi_message_auxiliary(CCL::IMessage* p) { return &p->getArg(2); }
extern "C" CCL::IPlugInManager* abi_plugin_manager_export() { return &CCL::System::GetPlugInManager(); }
extern "C" CCL::INativeFileSystem* abi_file_system_export() { return &CCL::System::GetFileSystem(); }
static_assert(std::is_base_of_v<CCL::IClassAllocator,CCL::IPlugInManager>);
static_assert(std::is_base_of_v<CCL::IFileSystem,CCL::INativeFileSystem>);
extern "C" CCL::tbool abi_object_invoke(CCL::IObject* p,CCL::Variant* out,CCL::IMessage* msg) { return p->invokeMethod(*out,*msg); }
extern "C" CCL::IString* abi_string_clone(CCL::IString* p) { return p->cloneString(); }
extern "C" CCL::tresult abi_string_assign(CCL::IString* p,const CCL::uchar* s,int n) { return p->assignChars(s,n); }
extern "C" CCL::IParameter* abi_controller_find(CCL::IController* p,const CCL::CString* name) { return p->findParameter(*name); }
extern "C" const CCL::ITypeInfo* abi_type_parent(CCL::ITypeInfo* p) { return p->getParentType(); }
extern "C" const CCL::ITypeInfo::MethodDefinition* abi_type_methods(CCL::ITypeInfo* p) { return p->getMethodNames(); }
extern "C" CCL::ModuleRef abi_type_module(CCL::ITypeInfo* p) { return p->getModuleReference(); }
extern "C" void abi_scripting_startup(CCL::IScriptingManager* p,const CCL::CString* id,CCL::ModuleRef module) { p->startup(*id,module,nullptr,false); }
extern "C" void abi_scripting_shutdown(CCL::IScriptingManager* p,CCL::ModuleRef module) { p->shutdown(module,false); }
extern "C" CCL::tbool abi_storable_save(CCL::IStorable* p,CCL::IStream* stream) { return p->save(*stream); }
extern "C" CCL::tbool abi_storable_load(CCL::IStorable* p,CCL::IStream* stream) { return p->load(*stream); }
extern "C" int abi_stream_read(CCL::IStream* p,void* buffer,int size) { return p->read(buffer,size); }
extern "C" int abi_stream_write(CCL::IStream* p,const void* buffer,int size) { return p->write(buffer,size); }
extern "C" CCL::int64 abi_stream_tell(CCL::IStream* p) { return p->tell(); }
extern "C" CCL::tbool abi_stream_seekable(CCL::IStream* p) { return p->isSeekable(); }
extern "C" CCL::int64 abi_stream_seek(CCL::IStream* p,CCL::int64 offset,int mode) { return p->seek(offset,mode); }

#ifdef LAYOUT_EXECUTABLE
int main() {
  std::puts("Public CCL provider layouts checked: Variant16 nontrivial, CString16, ThreadID8; all offsets checked");
}
#endif
