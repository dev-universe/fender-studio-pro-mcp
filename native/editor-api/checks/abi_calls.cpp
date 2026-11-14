// Independently authored compiler-only calls; no SDK source is copied or linked.
#include <cstddef>
#include <type_traits>
#include <utility>
#include "ccl/public/base/iobjectnode.h"
#include "ccl/public/base/iobject.h"
#include "ccl/public/gui/framework/iwindow.h"
#include "ccl/public/gui/framework/iview.h"
#include "ccl/public/textservices.h"
static_assert(sizeof(CCL::tbool)==1 && sizeof(CCL::tresult)==4);
static_assert(sizeof(CCL::UIDBytes)==16 && alignof(CCL::UIDBytes)==4);
static_assert(offsetof(CCL::UIDBytes,data1)==0 && offsetof(CCL::UIDBytes,data2)==4 && offsetof(CCL::UIDBytes,data3)==6 && offsetof(CCL::UIDBytes,data4)==8);
static_assert(sizeof(CCL::PlainString)==16 && alignof(CCL::PlainString)==8);
static_assert(std::is_same_v<CCL::StringRef,const CCL::String&>);
static_assert(std::is_same_v<decltype(std::declval<const CCL::IControl&>().getParameter()), CCL::IParameter*>);
extern "C" CCL::IParameter* abi_control_parameter(const CCL::IControl* p){return p->getParameter();}
extern "C" CCL::tresult abi_query(CCL::IUnknown* p,const CCL::UIDBytes* iid,void** out){return p->queryInterface(*iid,out);}
extern "C" unsigned abi_retain(CCL::IUnknown* p){return p->retain();}
extern "C" unsigned abi_release(CCL::IUnknown* p){return p->release();}
extern "C" const CCL::ITypeInfo* abi_object_type(CCL::IObject* p){return &p->getTypeInfo();}
extern "C" const char* abi_type_name(CCL::ITypeInfo* p){return p->getClassName();}
extern "C" const CCL::UIDBytes* abi_node_uid(CCL::IObjectNode* p){return &p->getObjectUID();}
extern "C" const CCL::UIDBytes* abi_node_class(CCL::IObjectNode* p){return &p->getClassUID();}
extern "C" CCL::IObjectNode* abi_node_find(CCL::IObjectNode* p,const CCL::String* key){return p->findChild(*key);}
extern "C" CCL::IObjectNode* abi_node_lookup(CCL::IObjectNode* p,const CCL::String* key){return p->lookupChild(*key);}
extern "C" int abi_desktop_count(CCL::IDesktop* p){return p->countWindows();}
extern "C" CCL::IWindow* abi_desktop_window(CCL::IDesktop* p,int i){return p->getWindow(i);}
extern "C" CCL::IUnknown* abi_window_controller(CCL::IWindow* p){return p->getController();}
extern "C" CCL::IUnknown* abi_view_controller(CCL::IView* p){return p->getController();}
extern "C" CCL::IViewChildren* abi_view_children(CCL::IView* p){return &p->getChildren();}
extern "C" CCL::IViewIterator* abi_iterator_create(CCL::IViewChildren* p){return p->createIterator();}
extern "C" CCL::tbool abi_direct_child(CCL::IViewChildren* p,CCL::IView* child,CCL::tbool deep){return p->isChildView(child,deep);}
extern "C" CCL::tbool abi_iterator_done(CCL::IViewIterator* p){return p->done();}
extern "C" CCL::IView* abi_iterator_next(CCL::IViewIterator* p){return p->next();}
extern "C" const CCL::String* abi_constant_string(const char* text){return &CCL::System::GetConstantString(text);}
