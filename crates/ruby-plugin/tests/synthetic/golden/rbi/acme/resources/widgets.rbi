# typed: strong

module Acme
  module Resources
    class Widgets
      sig { returns(Acme::Resources::Widgets::Parts) }
      attr_reader :parts

      # Creates a widget with the given name and parts. The SKU is assigned by the
      # server.
      sig { params(name: String, color: Acme::WidgetCreateParams::Color::OrSymbol, dimensions: Acme::WidgetCreateParams::Dimensions::OrHash, tags: T::Array[String], request_options: Acme::RequestOptions::OrHash).returns(Acme::Widget) }
      def create(
        # The widget's display name.
        name:,
        color: nil,
        dimensions: nil,
        tags: nil,
        request_options: {}
      )
      end

      # Retrieve a widget
      sig { params(widget_id: String, request_options: Acme::RequestOptions::OrHash).returns(Acme::Widget) }
      def retrieve(widget_id, request_options: {})
      end

      # List widgets
      sig { params(colors: T::Array[String], created_after: Time, limit: Integer, request_options: Acme::RequestOptions::OrHash).returns(Acme::Models::WidgetListResponse) }
      def list(colors: nil, created_after: nil, limit: nil, request_options: {})
      end

      # Delete a widget
      sig { params(widget_id: String, request_options: Acme::RequestOptions::OrHash).void }
      def delete(widget_id, request_options: {})
      end

      # @api private
      sig { params(client: Acme::Client).returns(T.attached_class) }
      def self.new(client:)
      end
    end
  end
end
