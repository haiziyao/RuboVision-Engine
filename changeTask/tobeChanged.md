1. 配置文件过于复杂，直接传递一个结构体，我们在config/type.rs中定义一些结构体，用于传递参数
2. 对于uart通信，目前定位不清晰，为什么uart属于一种设备，这是错误的，我们应该抽象成message层
把web和uart都抽象为message层，分别继承Message类，实现send方法，发往不同地方，
然后把web.yaml改为message.yaml，配置web和uart
3. 对于Source，目前需要实现一个假的GPIO，用于调试,叫做DebugSource，这个直接劫持gpio的sender，替gpio发送，然后连接到webSource,由web发信号(web界面需要同时更改,流出位置)，需要注意格式正确
4. 把function部分尤其需要改造一下，对于目前实现，尤其不行，
function部分应该 完全剥离： 
    
    * 首先对FunctionWorker改造，统一为pre_func,func,after_func。在after中统一做返回，比如web返回和gpio返回等判断，在pre中做一参数的解析，统一使用结构体解析(这里可以向使用者提问，请调研一下目前的实现优越还是用结构体解析优越)
    * 对于最终的效果，是我只需要在functions.rs中生命需要使用的函数，在配置文件中定义一下，就能自动返回，对于这个文件中的func，返回值统一为一个结构体，结构体包含一个returnEnum,returnWebEnum,returnGPIOEnum（gpio和web可以继承message）。然后由after函数去处理返回
    * 对于return 的channel，应该有FunctionFactory掌握，在初始化的时候就分配给它。
    * 对于如何实现只写函数，FunctionFactory就能自动发现，需要你调研方法，并向主人提问
    * 对目前usual.rs中的函数进行迁移到functions.rs。对于原来的`into_web_message`这些方法，按照我上面说的进行迁移。最后的functions.rs一定是非常干净的，只有我要实现的最终函数

5. 请使用前端skills，对前端页面重构，但是要保持所有功能不能失效

最终达成的效果就是，可以仅仅通过写函数+配置文件，达到自动调用函数的功能。

