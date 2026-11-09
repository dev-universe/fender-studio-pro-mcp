#include <cstddef>
#include <cstdio>
#include <type_traits>
#include <dlfcn.h>
#include <mach-o/loader.h>
#include "ccl/public/plugins/iclassfactory.h"
#include "ccl/public/plugins/icomponent.h"
#include "ccl/main/cclmodmain.h"

// Independently authored layout assertions. Public headers are not copied here.
static_assert(sizeof(CCL::tbool) == 1);
static_assert(sizeof(CCL::tresult) == 4);
static_assert(sizeof(CCL::ModuleEntryReason) == 4);
static_assert(sizeof(CCL::ModuleRef) == 8);
static_assert(sizeof(CCL::UIDBytes) == 16);
static_assert(alignof(CCL::UIDBytes) == 4);
static_assert(offsetof(CCL::UIDBytes, data1) == 0);
static_assert(offsetof(CCL::UIDBytes, data2) == 4);
static_assert(offsetof(CCL::UIDBytes, data3) == 6);
static_assert(offsetof(CCL::UIDBytes, data4) == 8);
static_assert(sizeof(CCL::PlainString) == 16);
static_assert(alignof(CCL::PlainString) == 8);
static_assert(offsetof(CCL::PlainString, theString) == 0);
static_assert(offsetof(CCL::PlainString, __private) == 8);
static_assert(sizeof(CCL::String) == sizeof(CCL::PlainString));
static_assert(alignof(CCL::String) == alignof(CCL::PlainString));
static_assert(std::is_standard_layout_v<CCL::String>);
// A standard-layout derived object and its sole base share the same address.
static_assert(sizeof(CCL::VersionDesc) == 80);
static_assert(alignof(CCL::VersionDesc) == 8);
static_assert(offsetof(CCL::VersionDesc, name) == 0);
static_assert(offsetof(CCL::VersionDesc, version) == 16);
static_assert(offsetof(CCL::VersionDesc, vendor) == 32);
static_assert(offsetof(CCL::VersionDesc, copyright) == 48);
static_assert(offsetof(CCL::VersionDesc, url) == 64);
static_assert(sizeof(CCL::ClassDesc) == 88);
static_assert(alignof(CCL::ClassDesc) == 8);
static_assert(offsetof(CCL::ClassDesc, flags) == 0);
static_assert(offsetof(CCL::ClassDesc, classID) == 4);
static_assert(offsetof(CCL::ClassDesc, category) == 24);
static_assert(offsetof(CCL::ClassDesc, name) == 40);
static_assert(offsetof(CCL::ClassDesc, subCategory) == 56);
static_assert(offsetof(CCL::ClassDesc, description) == 72);
static_assert(sizeof(CCL::IUnknown) == 8);
static_assert(sizeof(CCL::IClassFactory) == 8);
static_assert(sizeof(CCL::IComponent) == 8);
static_assert(sizeof(Dl_info)==32 && alignof(Dl_info)==8);
static_assert(offsetof(Dl_info,dli_fname)==0 && offsetof(Dl_info,dli_fbase)==8);
static_assert(offsetof(Dl_info,dli_sname)==16 && offsetof(Dl_info,dli_saddr)==24);
static_assert(sizeof(mach_header_64)==32 && offsetof(mach_header_64,ncmds)==16 && offsetof(mach_header_64,sizeofcmds)==20);
static_assert(sizeof(uuid_command)==24 && offsetof(uuid_command,uuid)==8);

int main() {
  std::puts("CCL public arm64 ABI: UID=16/align4, String=16/align8, VersionDesc=80, ClassDesc=88; all offsets checked");
}
