// Independent compiler-only interface calls. No SDK code is linked or shipped.
#include <cstddef>
#include <type_traits>
#include "ccl/public/base/iobject.h"
#include "ccl/public/base/iobjectnode.h"
#include "ccl/public/text/istring.h"
#include "ccl/public/textservices.h"
static_assert(sizeof(CCL::UIDBytes)==16 && alignof(CCL::UIDBytes)==4);
static_assert(sizeof(CCL::PlainString)==16 && alignof(CCL::PlainString)==8);
static_assert(sizeof(CCL::DataType)==4 && sizeof(CCL::tbool)==1);
static_assert(sizeof(CCL::ITypeInfo::MethodDefinition)==24 && alignof(CCL::ITypeInfo::MethodDefinition)==8);
static_assert(offsetof(CCL::ITypeInfo::MethodDefinition,name)==0 && offsetof(CCL::ITypeInfo::MethodDefinition,args)==8 && offsetof(CCL::ITypeInfo::MethodDefinition,retval)==16);
static_assert(sizeof(CCL::ITypeInfo::PropertyDefinition)==32 && alignof(CCL::ITypeInfo::PropertyDefinition)==8);
static_assert(offsetof(CCL::ITypeInfo::PropertyDefinition,name)==0 && offsetof(CCL::ITypeInfo::PropertyDefinition,type)==8 && offsetof(CCL::ITypeInfo::PropertyDefinition,typeName)==16 && offsetof(CCL::ITypeInfo::PropertyDefinition,typeInfo)==24);
static_assert(std::is_same_v<CCL::StringRef,const CCL::String&>);
extern "C" CCL::tresult abi_query(CCL::IUnknown* p,const CCL::UIDBytes* iid,void** out){return p->queryInterface(*iid,out);}
extern "C" unsigned abi_retain(CCL::IUnknown* p){return p->retain();}
extern "C" unsigned abi_release(CCL::IUnknown* p){return p->release();}
extern "C" const CCL::ITypeInfo* abi_object_type(CCL::IObject* p){return &p->getTypeInfo();}
extern "C" const CCL::ITypeInfo* abi_type_parent(CCL::ITypeInfo* p){return p->getParentType();}
extern "C" int abi_type_flags(CCL::ITypeInfo* p){return p->getClassFlags();}
extern "C" const char* abi_type_name(CCL::ITypeInfo* p){return p->getClassName();}
extern "C" const char* abi_type_namespace(CCL::ITypeInfo* p){return p->getClassNamespace();}
extern "C" const CCL::UIDBytes* abi_type_id(CCL::ITypeInfo* p){return &p->getClassID();}
extern "C" const CCL::ITypeInfo::MethodDefinition* abi_type_methods(CCL::ITypeInfo* p){return p->getMethodNames();}
extern "C" const CCL::ITypeInfo::PropertyDefinition* abi_type_properties(CCL::ITypeInfo* p){return p->getPropertyNames();}
extern "C" const CCL::String* abi_node_id(CCL::IObjectNode* p){return &p->getObjectID();}
extern "C" const CCL::UIDBytes* abi_node_uid(CCL::IObjectNode* p){return &p->getObjectUID();}
extern "C" const CCL::UIDBytes* abi_node_class(CCL::IObjectNode* p){return &p->getClassUID();}
extern "C" CCL::IObjectNode* abi_node_parent(CCL::IObjectNode* p){return p->getParent();}
extern "C" int abi_node_count(CCL::IObjectNode* p){return p->countChildren();}
extern "C" CCL::IObjectNode* abi_node_child(CCL::IObjectNode* p,int i){return p->getChild(i);}
extern "C" CCL::IObjectNode* abi_node_find(CCL::IObjectNode* p,const CCL::String* id){return p->findChild(*id);}
extern "C" int abi_string_length(CCL::IString* p){return p->getLength();}
extern "C" CCL::tresult abi_string_copy(CCL::IString* p,CCL::uchar* out,int n){return p->copyTo(out,n);}
extern "C" const CCL::String* abi_constant_string(const char* text){return &CCL::System::GetConstantString(text);}
