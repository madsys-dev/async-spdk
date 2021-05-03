#!/bin/sh -e

# This is a script which automates building of spdk fat library containing all
# spdk, dpdk and isa-l object files. It would have been nice if this was part
# of spdk makefiles so that anyone could run just configure && make to get the
# fat lib. But it's not a hot candidate for upstreaming since we do this only
# to work around limitations of rust build system, which is not a good reason
# for changing spdk makefiles.
#
# Usage: ./build.sh [extra-spdk-configure-args...]
#  (i.e. ./build.sh --enable-debug)

BASEDIR=$(dirname "$0")
cd $BASEDIR

# checkout spdk sources
[ -d spdk/.git ] || git submodule update --init --recursive

# *should* not be needed to specify the -nmno-xxx however for certain
# it does. There are some existing bugs out there where either
# or gcc do not fully do the right thing, general consensus is that
# you are "tuning" CPU flags, you know what you are doing so these issues
# not treated with high priority.
#
# Ideally, we would use -march=x86-64, however DPDK *requires* at least
#.1 corei7 is the first CPU family that supports this.
#
# The current supported CPU extensions we use are:
#
# MODE64 (call)
# CMOV (cmovae)
# AVX (vmovdqa)
# NOVLX (vmovntdq)
# SSE1 (sfence)
# SSE2 (pause)
# SSSE3 (palignr)
# PCLMUL (pclmulqdq)
# SSE41 (pblendvb)
#
# To see the flags enabled per -march=$arg, you can run:
#
#  gcc -Q -march=corei7 --help=target
#
#  It will show the information in a human readable format, to show the
#  preprocessor output:
#
#  gcc -E -dM -march=corei7 - < /dev/null
#
# note that neither the CPU extensions nor the GCC defines, will map directly
# to a CPU instruction, for this one really needs to read the manual
DISABLED_FLAGS="-mno-movbe -mno-lzcnt -mno-bmi -mno-bmi2"

# cd spdk
# CFLAGS=$DISABLED_FLAGS DPDK_EXTRA_FLAGS="-march=corei7 $DISABLED_FLAGS" ./configure \
#     --with-iscsi-initiator \
#     --with-rdma \
#     --with-internal-vhost-lib \
#     --disable-tests \
#     "$@"
# TARGET_ARCHITECTURE=corei7 make -j $(nproc)
# cd ..

ARCHIVES=
for f in spdk/build/lib/libspdk_*.a; do
	# avoid test mock lib with undefined symbols
	if [ "$f" != spdk/build/lib/libspdk_ut_mock.a ]; then
		ARCHIVES="$ARCHIVES $f"
	fi
done

for f in spdk/dpdk/build/lib/librte_*.a; do
	# avoid name clashes - spdk has its own vhost implementation
	# if [ "$f" != spdk/dpdk/build/lib/librte_vhost.a ]; then
		ARCHIVES="$ARCHIVES $f"
	# fi
done

# ARCHIVES="$ARCHIVES spdk/isa-l/.libs/libisal.a"

echo
echo "Constructing libspdk_fat.so from following object archives:"
for a in $ARCHIVES; do
	echo "    $a"
done

[ -d target ] || mkdir target
cc -shared -o target/libspdk_fat.so \
    -laio -lnuma -luuid -lcrypto \
	-Wl,--whole-archive $ARCHIVES -Wl,--no-whole-archive

	# -lc -lrdmacm -laio -libverbs -liscsi -lnuma -ldl -lrt -luuid -lcrypto \
echo
echo "Don't forget to install target/libspdk_fat.so to dir where it can be found by linker"
echo