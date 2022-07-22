use super::*;
use crate::utils::ColorUtils;
use egui::*;
use epaint::RectShape;

/// A node inside the [`Graph`]. Nodes have input and output parameters, stored
/// as ids. They also contain a custom `NodeData` struct with whatever data the
/// user wants to store per-node.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct ColumnNode<Content, InputPort, OutputPort> {
    pub position: Pos2,
    pub label: String,
    pub content: Content,
    /// The input ports of the graph
    pub inputs: SlotMap<InputId, InputPort>,
    /// The [`OutputParam`]s of the graph
    pub outputs: SlotMap<OutputId, OutputPort>,

    /// The size hint is used to automatically scale the widget to a desirable
    /// size while still allowing right-side ports to be justified to the right
    /// size of the node widget. If the desired size of a widget inside of the
    /// node's frame changes then the node size should be fixed after one bad
    /// rendering cycle.
    pub size_hint: f32,
}

pub type SimpleColumnNode<Content, DataType> =
    ColumnNode<Content, VerticalInputPort<DataType>, VerticalPort<DataType>>;

impl<Content, InputPort, OutputPort> NodeTrait for ColumnNode<Content, InputPort, OutputPort>
where
    Content: NodeContentTrait,
    Content::DataType: DataTypeTrait,
    Content::DataType::Value: WidgetValueTrait,
    InputPort: PortTrait,
    OutputPort: PortTrait,
{
    type Content = Content;

    fn show<Node>(
        &mut self,
        parent_ui: &mut egui::Ui,
        id: NodeId,
        state: NodeUiState<DataTypeOf<Node>>,
        graph: &Graph<Node>,
        context: &dyn GraphContext,
    ) -> Vec<NodeResponse<Self>>
    where
        Node: NodeTrait,
        Node::Content: NodeContentTrait<DataType=DataTypeOf<Self>>,
    {
        let mut ui = parent_ui.child_ui_with_id_source(
            Rect::from_min_size(self.position + state.pan, [self.size_hint, 0.0].into()),
            Layout::default(),
            id,
        );

        let margin = egui::vec2(15.0, 5.0);
        let mut responses = Vec::<NodeResponse<Self>>::new();

        let background_color = context.recommend_node_background_color(ui, state.node_id);
        let text_color = context.recommend_node_text_color(ui, state.node_id);

        ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.0, text_color);

        // Forward declare shapes to paint below contents
        let outline_shape = ui.painter().add(Shape::Noop);
        let background_shape = ui.painter().add(Shape::Noop);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = {
            let mut inner_rect = outer_rect_bounds.shrink2(margin);

            // Try to use the size hint, unless our outer limits are smaller
            inner_rect.max.x = inner_rect.max.x.min(self.size_hint);

            // Make sure we don't shrink to the negative
            inner_rect.max.x = inner_rect.max.x.max(inner_rect.min.x);
            inner_rect.max.y = inner_rect.max.y.max(inner_rect.min.y);

            inner_rect
        };

        let title_height;
        let mut child_ui = ui.child_ui(inner_rect, *ui.layout());
        child_ui.vertical(|ui| {
            let title_rect = ui.horizontal(|ui| {
                ui.add(Label::new(
                    RichText::new(self.label)
                        .text_style(TextStyle::Button)
                        .color(context.recommend_node_text_color(ui, state.node_id)),
                ));
                ui.add_space(8.0); // The size of the little cross icon
            }).response.rect;
            self.size_hint = title_rect.width();
            title_height = title_rect.height();

            for (input_id, port) in &self.inputs {
                ui.horizontal(|ui| {
                    let (rect, port_responses): (egui::Rect, Vec<PortResponse>) = port.show(
                        ui, (id, PortId::Input(input_id)), &state, context
                    );
                    responses.extend(port_responses.into_iter().map(NodeResponse::Port));
                    self.size_hint = self.size_hint.max(rect.width());
                });
            }

            for (output_id, port) in &self.outputs {
                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    let (rect, port_responses): (egui::Rect, Vec<PortResponse>) = port.show(
                        ui, (id, PortId::Output(output_id)), &state, context
                    );
                    responses.extend(port_responses.into_iter().map(NodeResponse::Port));
                    self.size_hint = self.size_hint.max(rect.width());
                });
            }

            responses.extend(self.content.content_ui(ui, id, graph).into_iter());
        });

        let (shape, outline) = {
            let rounding_radius = 4.0;
            let rounding = Rounding::same(rounding_radius);

            let outer_rect = child_ui.min_rect().expand2(margin);
            let titlebar_height = title_height + margin.y;
            let titlebar_rect = Rect::from_min_size(
                outer_rect.min, vec2(outer_rect.width(), titlebar_height)
            );
            let titlebar = Shape::Rect(RectShape{
                rect: titlebar_rect,
                rounding,
                fill: self.content.titlebar_color(
                    ui, id, graph
                ).unwrap_or_else(|| context.recommend_node_background_color(
                    ui, id).lighten(0.8)
                ),
                stroke: Stroke::none(),
            });

            let body_rect = Rect::from_min_size(
                outer_rect.min + vec2(0.0, titlebar_height - rounding_radius),
                vec2(outer_rect.width(), outer_rect.height() - titlebar_height),
            );
            let body = Shape::Rect(RectShape{
                rect: body_rect,
                rounding: Rounding::none(),
                fill: background_color,
                stroke: Stroke::none(),
            });

            let bottom_body_rect = Rect::from_min_size(
                body_rect.min + vec2(0.0, body_rect.height() - titlebar_height * 0.5),
                vec2(outer_rect.width(), title_height),
            );
            let bottom_body = Shape::Rect(RectShape {
                rect: bottom_body_rect,
                rounding,
                fill: background_color,
                stroke: Stroke::none(),
            });

            let outline = if self.selected {
                Shape::Rect(RectShape {
                    rect: titlebar_rect
                        .union(body_rect)
                        .union(bottom_body_rect)
                        .expand(1.0),
                    rounding,
                    fill: Color32::WHITE.lighten(0.8),
                    stroke: Stroke::none(),
                })
            } else {
                Shape::Noop
            };

            (Shape::Vec(vec![titlebar, body, bottom_body]), outline)
        };

        ui.painter().set(background_shape, shape);
        ui.painter().set(outline_shape, outline);

    }
}
