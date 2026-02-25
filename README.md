## RoboCup 2026 视觉代码
### 项目简介
2026 RoboCup车型竞技机器人视觉组代码。部署在树莓派5的 Rust + OpenCV项目。集成颜色识别，二维码识别，GPIO状态灯，UART串口通信，cross检测。

### 项目版本简介
本项目维护有两个大的子版本偶系列和奇系列，此外还有仅供参考的0系列。

目前已发布的0.1.0系项目的第一个版本，是整个项目的概览。偶系列和奇系列的版本更迭不受0版本主宰，但0版本发布新版本时，奇偶版本需要做出大版本更新。

#### 奇版本
* 功能与设备相关联：在项目启动时，设备进行硬注册，后续每次调用相同功能所使用的设备相同

#### 偶版本
* 功能与设备不关联:采用动态代理解耦，在项目启动时，设备进行软注册，后续根据通信信息进行调用

### 开发部署

#### 推荐正常部署流程
* 建议重装系统
* 重装系统的时候记得自己创建个人热点并连接，如果想知道为什么请见 [Before Cargo Run](#Before-Cargo-Run)
* 本项目采用树莓派5 (Debin13 Trixie版本而非Debian12 bookworm版本)
* 由于上一条的原因，各种库下载极其缓慢，所以我推荐使用代理
* 开发时我使用的是llvm-14.0.0,openCv 0.75的rust Crate，但是在部署树莓派的时候发现，默认安装的是新版的llvm,所以在部署的时候认为调整了opencv的版本。
* 遇到llvm版本不协调问题，注意优先修改Rust的OpenCV crate版本，不要擅自修改openCV SDK的版本(在系统中混装两个版本的OpenCV就是自讨苦吃)
``` bash
# OpenCV，-E表示不清空代理配置 
sudo -E apt install -y \
  build-essential pkg-config cmake git \
  clang libclang-dev llvm \
  libopencv-dev \
  v4l-utils

# 安装rust
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
rustc -V
cargo -V

# 查看自己的llvm是什么版本
# 注意如果版本过低或者过高出现报错，请切换openCV crate的版本
llvm-config --version

# 运行
cargo build

```
#### 作者开发环境
* ubuntu 22.04
* opencv_version: 4.5.4

##### 相关命令参考

``` bash
# 查看系统各种信息
hostnamectl
# 查看opencv版本
opencv_version
```

请注意，这并不代表作者推荐这个版本，请自行按照OpenCV Crate和OpenCV SDK的版本相匹配即可。
### Before Cargo Run
#### 树莓派的连接
* 在给树莓派装系统的时候，记得用自己手机或者电脑创建一个个人热点连接，为什么要这样？
  * 使用热点可以保障我们可以在任何地方都能调试树莓派
* 树莓派可以自动连接wifi
  * 注意不同wifi的区分只靠wifi名字，也就是说，你把电脑热点和手机热点都设置成相同的名字，都可以自动连接。
  * 也可以把热点的名字密码设置成地下室wifi，这样就很爽了，怎么都能自动连上wifi
* 使用树莓派自带的连接方法，见下。可以替代自己寻找ip然后连接
  * 最方便 使用mDNS
  * 只需要保证都在一个局域网就行了
* 当然，你也可以自己为树莓派设置一个静态ip，不过我还是推荐使用mDNS

``` bash
# 第一步： 
# 这个一般不用下载，树莓派默认已经启用服务了
# 使用服务先看一下是否在线
sudo systemctl status avahi-daemon
# 如果在线就不用管了
# 服务不在线一般就是没下载
sudo apt install avahi-daemon
sudo systemctl start avahi-daemon
sudo systemctl enable avahi-daemon

# 第二步
# 直接查找树莓派ip
ping hostname.local
# 比如我的就是 ping hzy.local
# 只要开启服务，位于 同一个局域网就能找到

# 第三步
# ssh连接
ssh username@hostname.local
# 我的就是ssh hzy@hzy.local
# 命令行中每次输入命令的那个开头一串东西就是 username@hostname
```
#### Change Config
* 配置文件位于`./config/param.toml`
  * 详细注释见toml文件
  
* 找到自己的摄像机设备

``` bash
# 方法一 :bash
ls /dev/video*
# 方法二 :使用v4l2-ctl工具()
sudo apt install v4l-utils
v4l2-ctl --list-devices
```
#### 支持命令行配置参数

``` bash
cargo run -- --key serial --value /dev/ttyS0
# 可简写，可多改
cargo run -- -k color_cam qr_cam -v /dev/video0 /dev/video4

# 目前支持有限
# "color_cam" => Some(("color_camera_config", "color_camera"))
# "qr_cam"    => Some(("qr_camera_config", "qr_camera"))
# "cross_cam" => Some(("cross_camera_config", "cross_camera"))
# "serial"    => Some(("gpio_config", "serial"))
```

### 功能概述
#### 颜色识别
其实，如果是对于一般的颜色识别，对于`如何排除环境干扰`是极其复杂的。

但是，对于我们比赛来说，这是一个极其简单的东西。我们的摄像头可以放在夹爪上面，这样我们需要检测的区域就是固定的了。所以我使用了ROI

此项目主要使用了`ROI`,`HSV筛选`,`面积筛选`
* 获取原始帧frame
* 使用ROI，只关注中间一个圆形区域，其他地方mask掩盖
* HSV筛选，得到mask
* 面积筛选，面积最大而且大于一定值(如0.85)才是我们想要的颜色
* 底层有硬编码计数器，当同一个颜色连续10次合格才会返回最终结果，否则无限循环
![调试图片](assets/img/color_detect/image-1.png)
![演示图片](assets/img/color_detect/image.png)

#### 二维码识别
* 使用OpenCV进行二维码识别需要额外的库函数
* 所以我们使用`quircs`库进行二维码扫描获取数据

#### GPIO
* 有三个状态信号灯，用来表示程序是否在运行，以及执行了什么任务

#### UART通信
目前还是硬编码
电控发送数据包 `a1` 就会出发执行颜色识别的代码
发送 `b1` 就会执行二维码识别