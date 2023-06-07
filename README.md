# FastWebdav
用FastAPI为WebDav提供数据。   
这个项目不是针对小白用户使用的，Alist用户不是目标用户，需要会Python   
之前的几个webdav项目都是通过原生rust实现，技术难度并不大，但是扩展性太差。后来想着用rust做个webdav然后数据通过api提供统一格式就可以自由的扩展了，所以才有了这个项目。
说到数据Python是获取数据的最佳选择了。才用FastAPI负责后端数据提供。具体接口信息可以看下:8000/docs里面的接口文档。   
# 使用
目前除了Docker还没有其它渠道使用该项目【Docker我还没做好，做好后放上地址】   
第一次访问后会生成providers.ini这个是数据提供的入口，里面的配置并不是目录的配置，以下是示例文件
```
[tmplink]
provider = TmpLink(provider="tmplink",token='XXXXXXXXXXXXXXXXXXXXXXXX')
name = tmplink
[stariver]
provider = Stariver(provider="stariver",token='XXXXXXXXXXXXXXXXXXXXXXXXXX')
name = stariver
[pikpak]
provider = PikPak(provider="pikpak",username='XXXXXXXXXXX',password='XXXXXXXXXXXX')
name = pikpak
[meting]
provider = Meting(provider="meting",server='netease',playlist_id='60198')
name = meting
```
上面的代码中定义了4个provider,里面是我内置的几个模型，供参考。   
配置字段[tmplink]这是唯一值相当于一个模型实例的名称   
provider这个的定义就是一个模型实例的代码，通过eval将这个字段的值转换成一个类的实例，里面的配置和参数可以在models目录下的类里自己处理
name字段是要目录文件夹的显示名字
## 新增新模型
大体框架已经搭好了，如果需要添加新模型，只需要在models目录内添加新的类即可。Webdav有很多操作目录还没实现，目前只做了列表和获取链接的接口.请求都是由webdav提供的固定格式。
```
def list_files(self, list_req:ListRequest): #这个是获取文件列表的方法返回DavFile列表即可
def get_url(self,dav_file:DavFile): #这个是获取文件链接的方法
```
下载链接有些可以在列表页面算出来就不用再实现了，里面有添加过期的方法有需要可以参考下   

其它操作稍后再完善吧，特别是上传，由于各个网站上传方法不一样，目前还没有想好要怎样固定数据格式。

# 再次重申 Alist用户不是目标用户