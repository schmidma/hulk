# Setup Yocto

Create a new directory for the build environment

```sh
mkdir yocto-vrohow
cd yocto-vrohow
```

Create new layer to customize the distribution

```sh
mkdir meta-vrohow
cd meta-vrohow
git init
```

Create a new `kas-project.yml` to define the new build environment

```yml
# kas-project.yml

header:
  version: 12
  includes:
    - repo: meta-nao
      file: kas-project.yml
repos:
  meta-nao:
    url: "https://github.com/hulks/meta-nao.git"
    refspec: e7150b4dc44bc289f4cf070acf0a04fd47d24a94
```

The `refspec` arguments can be any git refs.
At HULKs we pin the all repositories to fixed refs to ensure reproducible builds.
But the `refspec` can also point to any branch name (e.g. `main` or `langdale`).

Leave the new layer to enter the build environment top level

```sh
cd ..
# now again at [...]/yocto/
```

Download the `kas-container` startup script

```sh
curl -o kas-container -sL https://raw.githubusercontent.com/siemens/kas/master/kas-container
chmod u+x kas-container
```

The directory structure now looks like this:

```sh
yocto/
├── kas-container
└── meta-vrohow
    └── kas-project.yml
```

Now checkout the build environment with kas

```sh
./kas-container checkout meta-vrohow/kas-project.yml
```

Kas is now cloning all repositories specified in the `kas-project.yml` files.
Cloning these repositories can take some time as some of the git mirrors are tremendously slow.

To skip the configuration copy repositories from preparation:

```sh
cp -r ../yocto-vrohow-prep/meta-{clang/,congatec-x86/,intel/,openembedded/} ../yocto-vrohow-prep/poky .
```

The directoy structure of the build environment now contains all necessary layer repositories and a `build` directory which will contain the build artifacts.

```sh
.
├── build
│   └── conf
├── kas-container
├── meta-clang
│   └── [...]
├── meta-congatec-x86
│   └── [...]
├── meta-intel
│   └── [...]
├── meta-nao
│   └── [...]
├── meta-openembedded
│   └── [...]
├── meta-vrohow
│   └── kas-project.yml
└── poky
    └── [...]
```

To speed up the build process copy the `downloads` and `sstate-cache` directories

```sh
cp -r ~/worktree/yocto-vrohow-prep/build/{sstate-cache,downloads} build/
```

Copy the aldebaran robocupper opn file

```sh
cd meta-nao/recipes-support/aldebaran/
rsync -P hulk@bighulk.hulks.dev:shared_data/tools/softbank_2.8.5_for_Robocupers/nao-2.8.5.11_ROBOCUP_ONLY_with_root.opn .
```

Extract the aldebaran binaries to the `aldebaran-binaries` recipe.

```sh
mkdir aldebaran-binaries
./extract_binaries.sh -o aldebaran-binaries/aldebaran_binaries.tar.gz nao-2.8.5.11_ROBOCUP_ONLY_with_root.opn
```

Exit the recipe directory:

```sh
cd ../../../
# now again at [...]/yocto/
```

Enter the kas build container.

```sh
./kas-container shell meta-vrohow/kas-project.yml
```

In the build container the shell starts inside the build directory with yocto preconfigured.
The setup is complete and the image can be build:

```sh
bitbake nao-image
```

The build chain generated an flashable opn file at

```sh
ls build/tmp/deploy/images/nao-v6/nao-image-nao-core-minimal-1.0.ext3.gz.opn
```

## Download and Flash the Image

Download the generated image from the compile machine

```sh
rsync -LP rechenknecht:worktree/yocto-vrohow/build/tmp/deploy/images/nao-v6/nao-image-nao-core-minimal-1.0.ext3.gz.opn Downloads/
ls Downloads/nao-image-nao-core-minimal-1.0.ext3.gz.opn
```

Find a flashable USB stick.

```sh
lsblk
```

Flash the opn file to the USB stick.

```sh
sudo dd if=Downloads/nao-image-nao-core-minimal-1.0.ext3.gz.opn of=/dev/sdb bs=10M
sync
```

Insert the stick into the Nao and hold the chestbutton until it flashes blue (~5s).
At the end of the flashing procedure, the Nao reboots into the new image.

## Connect to the Nao

Find the IP address of the Nao. The default configuration is using DHCP.

```sh
nmap -p 22 10.1.24.0/24 | rg "nao"
ssh nao@10.1.24.197
cat /etc/os-release
```

## Build and Install an RPM package

Enter the build container

```sh
./kas-container shell meta-vrohow/kas-project.yml
```

To search existing recipes, use the [Layer Index](http://layers.openembedded.org/layerindex/branch/master/recipes/).
Inside the container, compile the package.

```sh
bitbake curl
```

Find, upload and install the RPM file to the nao

```sh
fd curl build/tmp/deploy/rpm/
scp maximilian@rechenknecht.hulks.dev:worktree/yocto-vrohow/build/tmp/deploy/rpm/corei7_64/curl-7.85.0-r0.corei7_64.rpm .
sudo rpm -i curl-7.85.0-r0.corei7_64.rpm
```

The nao has a freshly compiled `curl` executable installed.
To persist the installation of a recipe in the image, modify the image.

## Modify the Image

Add the `meta-vrohow` layer to the kas project:

```yml
# [...]
repos:
  meta-vrohow:
# [...]
```

Create a `conf/layer.conf` to specify build configuration for this layer.

```sh
mkdir conf
```

```bb
# conf/layer.conf

BBPATH .= ":${LAYERDIR}"
BBFILES += "\
            ${LAYERDIR}/recipes-*/*/*.bb \
            ${LAYERDIR}/recipes-*/*/*.bbappend \
            "

LAYERSERIES_COMPAT_vrohow = "langdale"

BBFILE_COLLECTIONS += "vrohow"
BBFILE_PATTERN_vrohow = "^${LAYERDIR}/"
```

Create an overlay for the `nao-image` recipe.

```sh
mkdir -p recipies-core/images
```

And add an overlay to the `nao-image`:

```bb
# recipes-core/images/nao-image.bbappend

CORE_IMAGE_EXTRA_INSTALL += "\
                             curl \
                            "
```

Now, reconfigure, rebuild and reflash the image to apply the modifications to the nao.

```sh
./kas-container shell meta-vrohow/kas-project.yml
bitbake nao-image
# download, flash, ...
```

## Build the SDK

To build the sdk, run

```sh
bitbake -c populate_sdk nao-image
```

Download the image from the build host and install it on the machine

```sh
rsync -LP rechenknecht:worktree/yocto-vrohow/build/tmp/deploy/sdk/nao-core-minimal-toolchain-1.0.sh .
./nao-core-minimal-toolchain-1.0.sh -d ./1.0
```
