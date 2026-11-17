#include <cstddef>
#include <type_traits>
#include "ccl/public/base/variant.h"
#include "ccl/public/gui/framework/iparametermenu.h"
#include "ccl/public/gui/framework/imenu.h"
#include "ccl/public/gui/iparameter.h"
static_assert(sizeof(CCL::tbool)==1 && sizeof(CCL::tresult)==4);
static_assert(sizeof(CCL::Variant)==16 && alignof(CCL::Variant)==8);
static_assert(offsetof(CCL::Variant,type)==0 && offsetof(CCL::Variant,lValue)==8);
extern "C" CCL::tresult menu_construct(CCL::IParameterMenuBuilder*p,CCL::IParameter*q){return p->construct(q);}
extern "C" CCL::IMenu* menu_build(CCL::IParameterMenuBuilder*p){return p->buildIMenu(nullptr);}
extern "C" const CCL::UIDBytes* menu_class(CCL::IMenu*p){return &p->getMenuClass();}
extern "C" int menu_count(CCL::IMenu*p){return p->countItems();}
extern "C" CCL::IMenuItem* menu_item(CCL::IMenu*p,int i){return p->getItem(i);}
extern "C" CCL::tbool menu_attr(CCL::IMenuItem*p,CCL::Variant*v,int i){return p->getItemAttribute(*v,i);}
extern "C" CCL::IMenu* menu_sub(CCL::IMenuItem*p){return p->getItemMenu();}
extern "C" CCL::tresult menu_create(CCL::IClassAllocator*p,const CCL::UIDBytes*c,const CCL::UIDBytes*i,void**o){return p->createInstance(*c,*i,o);}
