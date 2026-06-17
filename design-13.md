# [cc] 支持 macOS 触摸板捏合手势缩放

## Current state

缩放只有一条路：工具栏的 zoom in / zoom out / 重置按钮。三个按钮的 `@objc` 方法都在 `ViewerWindowController`（`Sources/AnyViewApp/ViewerWindowController.swift`），落到同一个私有方法 `setZoom(_:)`：

```swift
private func setZoom(_ value: CGFloat) {
    let snapped = (value * 10).rounded() / 10
    zoomLevel = min(max(snapped, Self.minZoom), Self.maxZoom)
    renderer?.setZoom(zoomLevel)
    zoomLabelButton?.title = zoomLabelText
}
```

缩放状态都在 controller 这一份：`zoomLevel`（私有，初值 1.0）、`minZoom` 0.5、`maxZoom` 3.0、`zoomStep` 0.1。夹上下限、把系数传给当前 renderer 的 `setZoom(_:)`、刷新工具栏百分比标签，全在这一个方法里。

renderer 那头，四个 renderer 都实现了 `ViewerRenderer.setZoom(_:)`：`PDFRenderer` 设 `pdfView.scaleFactor`、`ImageRenderer` 按系数缩 `imageView` 的 frame、`QuickLookRenderer` 和 `WebRenderer` 各自处理。

预览区是 `showWindow` 里建的视图层级：`DropTargetView` → `container`（`rendererContainer`）→ `renderer.view`。这一层目前没有任何手势处理。触摸板捏合时，事件要么被 renderer 自带的原生缩放吃掉（`PDFView`、`QLPreviewView`、`WKWebView` 都有），要么没人接。无论哪种，`zoomLevel` 都不会动，工具栏标签也不会跟着变。

不够用的地方：Mac 用户在触摸板上捏合是肌肉记忆，现在捏不动；就算某个 renderer 的原生捏合生效了，缩放的也是它自己内部那份比例，和 `ViewerWindowController.zoomLevel`、和工具栏标签对不上。

## Approach

目标：在预览区接住捏合手势，把它汇进 controller 已有的那条缩放通路，不另起一份缩放状态。

做法分三块。

**1. 在预览容器上接手势。** `DropTargetView` 重写 `magnify(with:)`。`NSMagnifyGestureRecognizer` 也能做，但 `DropTargetView` 已经是预览区的容器视图、已经在 `ViewerWindowController` 手里，重写 `magnify(with:)` 不用再加 recognizer 和 target/action 接线，少一层。`NSEvent.magnification` 给的是这次手势的增量（捏开为正、捏合为负）。`DropTargetView` 把这个增量回调出去，回调由 `ViewerWindowController` 在 `showWindow` 里设上，和现有的 `onDrop` 回调是同一个套路。

**2. 增量汇进 `zoomLevel`。** controller 收到增量后，用 `zoomLevel + 增量` 当新的目标系数，走和工具栏按钮同一个夹取-应用-刷新标签的逻辑。捏开增量为正、系数变大，捏合增量为负、系数变小，方向天然对。改的就是现有那份 `zoomLevel`，不新建第二份。

**3. 夹上下限、应用到 renderer、刷新标签。** 复用现有 `setZoom(_:)` 里的 `min(max(_, minZoom), maxZoom)` 和 `renderer?.setZoom(_:)` 和标签刷新。捏到底也越不过 0.5 / 3.0。

为了让测试能从一个真实的捏合事件一路走到真实 renderer 的 `setZoom`、中间不插替身，要开两个测试缝：
- 把 controller 上接手势增量的入口方法做成 internal（比如 `handleMagnification(_:)`），测试能直接喂增量；它内部仍调私有 `setZoom(_:)`，夹取和刷标签逻辑不暴露、不重复。
- 给 `zoomLevel` 和当前 renderer 加 internal 只读访问（`zoomLevel` 改成 `private(set)` 或加只读计算属性；renderer 加只读 getter），测试能读到捏合后的真实系数，并拿到真实 renderer 去验证 `setZoom` 真的应用上了。

`setZoom(_:)` 现有的 0.1 取整保留不动。Open question 里那条"连续缩放是否要绕过取整"本 issue 不做，验收只要方向对、夹在上下限内、和标签一致，取整不影响这三点。原生捏合和 app 层手势抢事件那条，留实现阶段处理；本设计按"缩放统一走 `ViewerWindowController`"来定。

## Acceptance criteria → tests

测试在 `Tests/AnyViewAppTests/` 新建 `PinchZoomTests.swift`，沿用现有 `WordCommentHTMLTests` 的 `@testable import AnyViewApp` + XCTest 风格。每个测试构造一个真实的 `ViewerWindowController`（用一个真实存在的临时文件路径，比如临时目录下的 `.png`，这样 `RendererFactory` 给出真实的 `ImageRenderer`/`PDFRenderer`），调 internal 的捏合入口喂增量，再读真实状态断言。

### 验收 1 — 预览区接住捏合并触发 controller 缩放
- Call chain: 触摸板捏合 → `DropTargetView.magnify(with:)` → 增量回调 → `ViewerWindowController.handleMagnification(_:)` → `setZoom(_:)`
- Test entry: `ViewerWindowController.handleMagnification(_:)`。测试不从 `DropTargetView.magnify(with:)` 起，因为 `NSEvent` 的捏合事件没有公开构造方式、没法在单元测试里造一个真的 magnify 事件；`magnify(with:)` 本身只做"读 `event.magnification`、转调回调"这一件事，测试从回调落点 `handleMagnification(_:)` 进，覆盖的是缩放逻辑全程。
- Test: `test_magnification_triggersControllerZoom` in `Tests/AnyViewAppTests/PinchZoomTests.swift`（喂一个正增量，断言 `zoomLevel` 从 1.0 变了，证明手势入口确实接到了 controller 的缩放）

### 验收 2 — 捏开变大 / 捏合变小，改的是同一份 zoomLevel
- Call chain: `ViewerWindowController.handleMagnification(_:)` → `setZoom(_:)` → 写 `zoomLevel`
- Test entry: `ViewerWindowController.handleMagnification(_:)`（不绕层）
- Test: `test_magnification_positiveIncreasesNegativeDecreasesSameZoomLevel` in `Tests/AnyViewAppTests/PinchZoomTests.swift`（喂正增量后读 `zoomLevel` 大于初值，再喂负增量后读 `zoomLevel` 变小；全程读的是同一个 `zoomLevel` 属性，证明没有第二份状态）

### 验收 3 — 捏合后的系数被 minZoom / maxZoom 夹住
- Call chain: `ViewerWindowController.handleMagnification(_:)` → `setZoom(_:)` → `min(max(_, minZoom), maxZoom)`
- Test entry: `ViewerWindowController.handleMagnification(_:)`（不绕层）
- Test: `test_magnification_clampsToMinAndMaxZoom` in `Tests/AnyViewAppTests/PinchZoomTests.swift`（喂一个大到会超过 3.0 的正增量，断言 `zoomLevel == maxZoom`；再喂一个大到会低于 0.5 的负增量，断言 `zoomLevel == minZoom`）

### 验收 4 — 最终调到真实 renderer 的 setZoom，不经替身
- Call chain: `ViewerWindowController.handleMagnification(_:)` → `setZoom(_:)` → `renderer?.setZoom(_:)`（真实 `PDFRenderer.setZoom`）→ `pdfView.scaleFactor`
- Test entry: `ViewerWindowController.handleMagnification(_:)`。renderer 是 `RendererFactory` 给的真实实例，不是 stub。用 `PDFRenderer`，因为它的 `setZoom` 有可读的外部效果（`pdfView.scaleFactor`），断言能落到具体值。
- Test: `test_magnification_appliesToRealRendererSetZoom` in `Tests/AnyViewAppTests/PinchZoomTests.swift`（controller 持一个真实 `PDFRenderer`，喂增量后从 controller 的只读 renderer getter 取到它，断言 `pdfView.scaleFactor` 等于夹取后的 `zoomLevel`，证明缩放真的应用到了 renderer 上）

### 验收 5 — 捏合后工具栏百分比标签和 zoomLevel 一致
- Call chain: `ViewerWindowController.handleMagnification(_:)` → `setZoom(_:)` → 写 `zoomLabelButton.title`
- Test entry: `ViewerWindowController.handleMagnification(_:)`。标签按钮在 `showWindow` 建工具栏时才生成，测试先 `showWindow` 把工具栏装上，再喂增量。
- Test: `test_magnification_updatesToolbarPercentLabel` in `Tests/AnyViewAppTests/PinchZoomTests.swift`（`showWindow` 后喂增量，断言工具栏 zoom 标签按钮的 `title` 等于 `"\(Int((zoomLevel*100).rounded()))%"`，即标签文字和当前 `zoomLevel` 算出来一致）

## Risks & trade-offs

- **测试进不了 `magnify(with:)` 的最外层。** `NSEvent` 的 magnify 事件造不出来，所以测试从增量回调的落点 `handleMagnification(_:)` 起，`DropTargetView.magnify(with:)` 里"读 `event.magnification`、转调回调"这一小段进不了单元测试，只能靠手动在触摸板上试。这段逻辑很薄，但它没被自动覆盖，是个真实的盲点。
- **为测试放宽了访问级别。** `zoomLevel` 要从 `private` 放到 `private(set)`、加 internal 的捏合入口和 renderer getter。这是为了让测试不插桩就能读到真实状态。代价是 controller 的内部状态对同模块多露了几个口子；缩到 internal、保持只读能压住一部分，但确实比现在松。
- **原生捏合冲突没在本 issue 解决。** `PDFView` / `QLPreviewView` / `WKWebView` 自带捏合，可能和 `DropTargetView.magnify(with:)` 抢事件，导致某些 renderer 上手势缩放和工具栏标签对不齐。本设计按"统一走 controller"定方向，具体压制原生捏合留给实现阶段。如果实现阶段发现某个 renderer 压不住，验收 5 在那个 renderer 上可能表现不一致——届时要么在该 renderer 关掉原生捏合，要么把范围记进 follow-up。
- **某些 renderer 的 `setZoom` 接近空操作。** 出 scope 已说四个 renderer 都实现了 `setZoom`，空实现也不该崩。测试特意挑 `PDFRenderer`（`setZoom` 有可观察效果）验证应用，不依赖某个 renderer 内部一定改了什么。
