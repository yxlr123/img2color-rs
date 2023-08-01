# img2color
本项目使用Rust编写，具有较高的性能，~~应该吧~~
新螃蟹🦀的小练习，代码风格可能比较**混乱**

> 项目完善程度不高

## 部署
.env文件配置
| 配置项                  | 说明                                 |
|-------------------------|--------------------------------------|
| PORT                    | 端口 （默认3000）                            |

## api

只有一个~ `/api`

参数：

| 参数                  | 说明                                 |
|----------------------|--------------------------------------|
| img                  | (必填) 需要提取主题色的图片URL           |

返回示例：

``` json
{
    err: null,
    rgb: "#BC695A",
}
```

说明：

| 返回值                   | 说明                                 |
|-------------------------|--------------------------------------|
| err                     | 错误 （nil/string）                    |
| rgb                     | 主题色Hex（string）                    |
