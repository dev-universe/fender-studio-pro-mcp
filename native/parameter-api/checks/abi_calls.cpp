// Compiler-only probes against a separately obtained pinned public SDK.
// Independently authored declarations/calls; never linked into this Rust crate.
#include <cstddef>
#include <new>
#include <type_traits>
#include "ccl/public/base/iobject.h"
#include "ccl/public/base/iformatter.h"
#include "ccl/public/gui/iparameter.h"
#include "ccl/public/text/istring.h"
#include "ccl/public/text/cclstring.h"
#include "ccl/public/text/cstring.h"

static_assert(sizeof(CCL::tbool)==1 && sizeof(CCL::tresult)==4);
static_assert(sizeof(CCL::UIDBytes)==16 && alignof(CCL::UIDBytes)==4);
static_assert(sizeof(CCL::PlainString)==16 && alignof(CCL::PlainString)==8);
static_assert(offsetof(CCL::PlainString,theString)==0 && offsetof(CCL::PlainString,__private)==8);
static_assert(sizeof(CCL::String)==16 && std::is_standard_layout_v<CCL::String>);
static_assert(sizeof(CCL::PlainCString)==16 && alignof(CCL::PlainCString)==8);
static_assert(offsetof(CCL::PlainCString,text)==0 && offsetof(CCL::PlainCString,theString)==8);
static_assert(sizeof(CCL::CString)==16 && std::is_standard_layout_v<CCL::CString>);
static_assert(sizeof(CCL::Variant)==16 && alignof(CCL::Variant)==8);
static_assert(offsetof(CCL::Variant,type)==0 && offsetof(CCL::Variant,lValue)==8);
static_assert(!std::is_trivially_copyable_v<CCL::Variant>);
static_assert(!std::is_trivially_destructible_v<CCL::Variant>);
static_assert(std::is_same_v<CCL::StringID,const CCL::CString&>);
static_assert(std::is_same_v<CCL::VariantRef,const CCL::Variant&>);
static_assert(std::is_same_v<decltype(std::declval<CCL::IParameter&>().beginEdit()),void>);
static_assert(std::is_same_v<decltype(std::declval<CCL::IParameter&>().endEdit()),void>);
// The newer-SDK regression compares dispatch separately from changed flags.
#ifndef FENDER_PARAMETER_CHECK_OFFSETS_ONLY
static_assert(CCL::IParameter::kReadOnly==0x1000);
static_assert(CCL::IParameter::kStorable==0x80);
static_assert(CCL::IParameter::kStoreListValue==0x100);
static_assert(CCL::IParameter::kMutable==0x800);
static_assert(CCL::IParameter::kCanUndo==0x10000);
#endif
static_assert(CCL::kResultNoInterface==static_cast<CCL::tresult>(0x80004002u));
static_assert(CCL::IFormatter::kNormalized==1 && CCL::IFormatter::kStateful==2);

extern "C" CCL::tresult abi_query(CCL::IUnknown* p,const CCL::UIDBytes* iid,void** out) {return p->queryInterface(*iid,out);}
extern "C" unsigned abi_retain(CCL::IUnknown* p) {return p->retain();}
extern "C" unsigned abi_release(CCL::IUnknown* p) {return p->release();}
extern "C" int abi_parameter_type(CCL::IParameter* p) {return p->getType();}
extern "C" const CCL::CString* abi_parameter_name(CCL::IParameter* p) {return &p->getName();}
extern "C" CCL::tbool abi_parameter_enabled(CCL::IParameter* p) {return p->isEnabled();}
extern "C" CCL::tbool abi_parameter_state(CCL::IParameter* p,int mask) {return p->getState(mask);}
extern "C" int abi_parameter_tag(CCL::IParameter* p) {return p->getTag();}
extern "C" CCL::IUnknown* abi_parameter_controller(CCL::IParameter* p) {return p->getController();}
extern "C" void abi_parameter_begin_edit(CCL::IParameter* p) {p->beginEdit();}
extern "C" void abi_parameter_end_edit(CCL::IParameter* p) {p->endEdit();}
extern "C" CCL::IParameter* abi_parameter_original(CCL::IParameter* p) {return p->getOriginal();}
extern "C" int abi_parameter_precision(CCL::IParameter* p) {return p->getPrecision();}
extern "C" void abi_parameter_set_raw(CCL::IParameter* p,const CCL::Variant* v,CCL::tbool update) {p->setValue(*v,update);}
extern "C" float abi_parameter_normalized(CCL::IParameter* p) {return p->getNormalized();}
extern "C" void abi_parameter_set_normalized(CCL::IParameter* p,float n,CCL::tbool update) {p->setNormalized(n,update);}
extern "C" CCL::IFormatter* abi_parameter_formatter(CCL::IParameter* p) {return p->getFormatter();}
extern "C" void abi_parameter_choice_string(CCL::IParameter* p,CCL::String* out,const CCL::Variant* value) {p->getString(*out,*value);}
extern "C" void abi_parameter_to_string(CCL::IParameter* p,CCL::String* out) {p->toString(*out);}
extern "C" void abi_parameter_from_string(CCL::IParameter* p,const CCL::String* in,CCL::tbool update) {p->fromString(*in,update);}
extern "C" CCL::tbool abi_object_property(CCL::IObject* p,CCL::Variant* out,const CCL::CString* id) {return p->getProperty(*out,*id);}
extern "C" int abi_string_length(CCL::IString* p) {return p->getLength();}
extern "C" CCL::tresult abi_string_copy(CCL::IString* p,CCL::uchar* out,int n) {return p->copyTo(out,n);}
extern "C" CCL::IString* abi_string_clone(CCL::IString* p) {return p->cloneString();}
extern "C" decltype(((CCL::IString*)nullptr)->getPrivateData()) abi_string_private(CCL::IString* p) {return p->getPrivateData();}
extern "C" CCL::tresult abi_string_assign(CCL::IString* p,const CCL::uchar* in,int n) {return p->assignChars(in,n);}
extern "C" int abi_formatter_flags(CCL::IFormatter* p) {return p->getFlags();}
extern "C" CCL::tbool abi_formatter_scan(CCL::IFormatter* p,CCL::Variant* out,const CCL::String* in) {return p->scanString(*out,*in);}
// Explicit placement shows why the otherwise similar BY-VALUE getter is excluded.
extern "C" void abi_forbidden_value_return(CCL::IParameter* p,void* out) {new(out) CCL::Variant(p->getValue());}
extern "C" CCL::IString* abi_empty_export() {return &CCL::System::GetEmptyString();}
extern "C" const CCL::CString* abi_cstring_export(const char* text) {return &CCL::System::GetConstantCString(text);}
